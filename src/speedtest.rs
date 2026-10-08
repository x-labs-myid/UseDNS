//! HTTP throughput to Cloudflare, not an implementation of Ookla's algorithm.
use curl::easy::{Easy, List};
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

const STREAMS: usize = 4;
const WARMUP: Duration = Duration::from_secs(3);
const MEASURE: Duration = Duration::from_secs(10);

pub struct ResultValues {
    pub download: String,
    pub upload: String,
    pub latency: String,
}

fn session() -> Result<Easy, String> {
    let mut easy = Easy::new();
    easy.connect_timeout(Duration::from_secs(5))
        .map_err(error)?;
    easy.timeout(Duration::from_secs(20)).map_err(error)?;
    easy.fail_on_error(true).map_err(error)?;
    easy.progress(true).map_err(error)?;
    easy.signal(false).map_err(error)?;
    easy.low_speed_limit(1).map_err(error)?;
    easy.low_speed_time(Duration::from_secs(5)).map_err(error)?;
    // TLS peer and host verification retain libcurl's secure defaults.
    let mut headers = List::new();
    headers.append("Cache-Control: no-cache").map_err(error)?;
    headers
        .append("Content-Type: application/octet-stream")
        .map_err(error)?;
    headers.append("Expect:").map_err(error)?;
    easy.http_headers(headers).map_err(error)?;
    Ok(easy)
}

fn error(e: curl::Error) -> String {
    e.to_string()
}
fn cancelled(cancel: &AtomicBool) -> Result<(), String> {
    if cancel.load(Ordering::Relaxed) {
        Err("Cancelled".into())
    } else {
        Ok(())
    }
}

fn median(values: &mut [f64]) -> Result<f64, String> {
    if values.is_empty() || values.iter().any(|v| !v.is_finite() || *v < 0.0) {
        return Err("Invalid measurement samples".into());
    }
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    Ok(if values.len().is_multiple_of(2) {
        (values[middle - 1] + values[middle]) / 2.0
    } else {
        values[middle]
    })
}

fn latency(base: &str, cancel: &AtomicBool) -> Result<f64, String> {
    let mut easy = session()?;
    let mut samples = Vec::new();
    for sample in 0..7 {
        cancelled(cancel)?;
        easy.url(&format!("{base}/__down?bytes=0&sample={sample}"))
            .map_err(error)?;
        {
            let mut transfer = easy.transfer();
            transfer
                .write_function(|data| Ok(data.len()))
                .map_err(error)?;
            transfer
                .progress_function(|_, _, _, _| !cancel.load(Ordering::Relaxed))
                .map_err(error)?;
            let result = transfer.perform();
            cancelled(cancel)?;
            result.map_err(error)?;
        }
        // Keep HTTP response latency distinct from ICMP ping; exclude DNS/TCP/TLS.
        if sample > 0 {
            samples.push(
                easy.starttransfer_time()
                    .map_err(error)?
                    .saturating_sub(easy.pretransfer_time().map_err(error)?)
                    .as_secs_f64()
                    * 1000.0,
            );
        }
    }
    median(&mut samples)
}

fn mbps(bytes: u64, elapsed: Duration) -> f64 {
    bytes as f64 * 8.0 / elapsed.as_secs_f64().max(0.001) / 1_000_000.0
}

fn chunk_size(bytes: u64, elapsed: Duration) -> u64 {
    // Aim for roughly half-second requests, keeping memory use bounded.
    ((bytes as f64 / elapsed.as_secs_f64().max(0.001) * 0.5) as u64).clamp(64_000, 32_000_000)
}

fn worker(
    base: &str,
    upload: bool,
    cancel: &AtomicBool,
    stop: &AtomicBool,
    counter: &AtomicU64,
    stream: usize,
) -> Result<(), String> {
    let mut easy = session()?;
    let mut completed = 0;
    let mut size = 256_000;
    let mut sequence = 0;
    // Generate a bounded, nonzero payload without temporary files or compression.
    let mut payload = vec![0u8; 64 * 1024];
    let mut seed = 0x9e3779b9u32 ^ stream as u32;
    for byte in &mut payload {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        *byte = seed as u8;
    }
    if upload {
        easy.post(true).map_err(error)?;
    }
    while !stop.load(Ordering::Relaxed) && !cancel.load(Ordering::Relaxed) {
        let url = if upload {
            format!("{base}/__up?stream={stream}&sequence={sequence}")
        } else {
            format!("{base}/__down?bytes={size}&stream={stream}&sequence={sequence}")
        };
        easy.url(&url).map_err(error)?;
        if upload {
            easy.post_field_size(size).map_err(error)?;
        }
        let began = Instant::now();
        let mut sent = 0u64;
        let result = {
            let mut transfer = easy.transfer();
            transfer
                .write_function(|data| Ok(data.len()))
                .map_err(error)?;
            if upload {
                transfer
                    .read_function(|buffer| {
                        let count = (size - sent)
                            .min(buffer.len() as u64)
                            .min(payload.len() as u64) as usize;
                        buffer[..count].copy_from_slice(&payload[..count]);
                        sent += count as u64;
                        Ok(count)
                    })
                    .map_err(error)?;
            }
            transfer
                .progress_function(|_, down, _, up| {
                    let bytes = if upload { up } else { down };
                    counter.store(completed + bytes.max(0.0) as u64, Ordering::Relaxed);
                    !stop.load(Ordering::Relaxed) && !cancel.load(Ordering::Relaxed)
                })
                .map_err(error)?;
            transfer.perform()
        };
        if stop.load(Ordering::Relaxed) || cancel.load(Ordering::Relaxed) {
            return Ok(());
        }
        result.map_err(error)?;
        let bytes = if upload {
            easy.upload_size()
        } else {
            easy.download_size()
        }
        .map_err(error)? as u64;
        if bytes == 0 {
            return Err("Server returned an empty transfer".into());
        }
        completed += bytes;
        counter.store(completed, Ordering::Relaxed);
        size = chunk_size(bytes, began.elapsed());
        sequence += 1;
    }
    Ok(())
}

fn throughput(
    base: &str,
    upload: bool,
    cancel: &AtomicBool,
    warmup: Duration,
    measure: Duration,
    mut progress: impl FnMut(String),
) -> Result<f64, String> {
    let stop = AtomicBool::new(false);
    let counters: [_; STREAMS] = std::array::from_fn(|_| AtomicU64::new(0));
    let total = || {
        counters
            .iter()
            .map(|c| c.load(Ordering::Relaxed))
            .sum::<u64>()
    };
    std::thread::scope(|scope| {
        let (tx, rx) = mpsc::channel();
        let mut handles = Vec::new();
        for (stream, counter) in counters.iter().enumerate() {
            let tx = tx.clone();
            let stop = &stop;
            handles.push(scope.spawn(move || {
                let result = worker(base, upload, cancel, stop, counter, stream);
                let _ = tx.send(result);
            }));
        }
        drop(tx);
        let started = Instant::now();
        let mut baseline = None;
        let mut history = VecDeque::new();
        let mut last_update = Instant::now();
        let result = loop {
            if let Err(e) = cancelled(cancel) {
                break Err(e);
            }
            if let Some(e) = rx.try_iter().find_map(Result::err) {
                break Err(e);
            }
            let now = Instant::now();
            let bytes = total();
            if baseline.is_none() && now.duration_since(started) >= warmup {
                baseline = Some((now, bytes));
                history.push_back((now, bytes));
            }
            if let Some((began, initial)) = baseline {
                let elapsed = now.duration_since(began);
                if elapsed >= measure {
                    let value = mbps(bytes.saturating_sub(initial), elapsed);
                    break if value > 0.0 {
                        Ok(value)
                    } else {
                        Err("No data transferred during measurement".into())
                    };
                }
                if now.duration_since(last_update) >= Duration::from_millis(250) {
                    while history.len() > 1
                        && now.duration_since(history[1].0) >= Duration::from_secs(1)
                    {
                        history.pop_front();
                    }
                    if let Some(&(previous, count)) = history.front() {
                        progress(format!(
                            "{:.2}",
                            mbps(bytes.saturating_sub(count), now.duration_since(previous))
                        ));
                    }
                    history.push_back((now, bytes));
                    last_update = now;
                }
            }
            std::thread::sleep(Duration::from_millis(25));
        };
        // Stop every stream before joining, including on errors and cancellation.
        stop.store(true, Ordering::Relaxed);
        for handle in handles {
            if handle.join().is_err() {
                return Err("Transfer worker failed".into());
            }
        }
        cancelled(cancel)?;
        result
    })
}

pub fn run(
    cancel: &AtomicBool,
    mut progress: impl FnMut(i32, String),
) -> Result<ResultValues, String> {
    let base = "https://speed.cloudflare.com";
    let latency = format!("{:.0}", latency(base, cancel)?);
    progress(1, latency.clone());
    progress(2, "—".into());
    let download = format!(
        "{:.2}",
        throughput(base, false, cancel, WARMUP, MEASURE, |v| progress(2, v))?
    );
    progress(2, download.clone());
    progress(3, "—".into());
    let upload = format!(
        "{:.2}",
        throughput(base, true, cancel, WARMUP, MEASURE, |v| progress(3, v))?
    );
    progress(3, upload.clone());
    Ok(ResultValues {
        download,
        upload,
        latency,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{BufRead, BufReader, Read, Write},
        net::TcpListener,
        sync::Arc,
    };

    struct Server {
        base: String,
        stop: Arc<AtomicBool>,
        requests: Arc<AtomicU64>,
        thread: Option<std::thread::JoinHandle<()>>,
    }

    impl Server {
        fn new(fail: bool) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let base = format!("http://{}", listener.local_addr().unwrap());
            listener.set_nonblocking(true).unwrap();
            let stop = Arc::new(AtomicBool::new(false));
            let requests = Arc::new(AtomicU64::new(0));
            let stopped = stop.clone();
            let count = requests.clone();
            let thread = std::thread::spawn(move || {
                let mut handlers = Vec::new();
                while !stopped.load(Ordering::Relaxed) {
                    if let Ok((socket, _)) = listener.accept() {
                        let stopped = stopped.clone();
                        let count = count.clone();
                        handlers.push(std::thread::spawn(move || {
                            socket.set_nonblocking(false).unwrap();
                            socket.set_read_timeout(Some(Duration::from_millis(100))).unwrap();
                            socket.set_write_timeout(Some(Duration::from_millis(100))).unwrap();
                            let mut reader = BufReader::new(socket);
                            let mut buffer = [0u8; 16_384];
                            while !stopped.load(Ordering::Relaxed) {
                                let mut request = String::new();
                                if reader.read_line(&mut request).unwrap_or(0) == 0 { break; }
                                let mut length = 0usize;
                                loop {
                                    let mut line = String::new();
                                    if reader.read_line(&mut line).unwrap_or(0) == 0 { return; }
                                    if line == "\r\n" { break; }
                                    if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                                        length = value.trim().parse().unwrap();
                                    }
                                }
                                count.fetch_add(1, Ordering::Relaxed);
                                if fail {
                                    let _ = reader.get_mut().write_all(b"HTTP/1.1 500 Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                                    return;
                                }
                                let upload = request.starts_with("POST ");
                                while length > 0 {
                                    let take = length.min(buffer.len());
                                    if reader.read_exact(&mut buffer[..take]).is_err() { return; }
                                    length -= take;
                                    std::thread::sleep(Duration::from_millis(1));
                                }
                                let bytes = if upload { 0 } else {
                                    request.split("bytes=").nth(1).and_then(|v| v.split('&').next())
                                        .and_then(|v| v.parse::<usize>().ok()).unwrap_or(0)
                                };
                                std::thread::sleep(Duration::from_millis(3));
                                if write!(reader.get_mut(), "HTTP/1.1 200 OK\r\nContent-Length: {bytes}\r\nConnection: keep-alive\r\n\r\n").is_err() { return; }
                                let mut remaining = bytes;
                                while remaining > 0 && !stopped.load(Ordering::Relaxed) {
                                    let take = remaining.min(buffer.len());
                                    if reader.get_mut().write_all(&buffer[..take]).is_err() { return; }
                                    remaining -= take;
                                    std::thread::sleep(Duration::from_millis(1));
                                }
                            }
                        }));
                    } else {
                        std::thread::sleep(Duration::from_millis(2));
                    }
                }
                for handler in handlers {
                    let _ = handler.join();
                }
            });
            Self {
                base,
                stop,
                requests,
                thread: Some(thread),
            }
        }
    }
    impl Drop for Server {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
            self.thread.take().unwrap().join().unwrap();
        }
    }

    #[test]
    fn local_transfers_report_live_bytes_reuse_sessions_and_measure_latency() {
        let server = Server::new(false);
        let cancel = AtomicBool::new(false);
        let delay = latency(&server.base, &cancel).unwrap();
        assert!(delay > 0.0 && delay < 500.0);
        for upload in [false, true] {
            let mut updates = Vec::new();
            let speed = throughput(
                &server.base,
                upload,
                &cancel,
                Duration::from_millis(80),
                Duration::from_millis(650),
                |v| updates.push(v),
            )
            .unwrap();
            assert!(speed.is_finite() && speed > 0.0);
            assert!(updates.len() >= 2);
            assert!(updates.iter().any(|v| v.parse::<f64>().unwrap() > 0.0));
        }
        assert!(server.requests.load(Ordering::Relaxed) > 7 + STREAMS as u64 * 2);
    }

    #[test]
    fn cancellation_and_http_errors_stop_all_streams() {
        let server = Server::new(false);
        let cancel = AtomicBool::new(false);
        std::thread::scope(|scope| {
            scope.spawn(|| {
                std::thread::sleep(Duration::from_millis(300));
                cancel.store(true, Ordering::Relaxed);
            });
            let started = Instant::now();
            assert_eq!(
                throughput(
                    &server.base,
                    false,
                    &cancel,
                    Duration::from_millis(50),
                    MEASURE,
                    |_| {}
                )
                .unwrap_err(),
                "Cancelled"
            );
            assert!(started.elapsed() < Duration::from_secs(3));
        });
        let failed = Server::new(true);
        let started = Instant::now();
        assert!(
            throughput(
                &failed.base,
                false,
                &AtomicBool::new(false),
                Duration::from_millis(50),
                MEASURE,
                |_| {}
            )
            .is_err()
        );
        assert!(started.elapsed() < Duration::from_secs(3));
    }
    #[test]
    fn robust_samples_and_units() {
        assert_eq!(median(&mut [2.0, 100.0, 4.0, 3.0]).unwrap(), 3.5);
        assert!(median(&mut [f64::NAN]).is_err());
        assert!(median(&mut []).is_err());
        assert_eq!(mbps(12_500_000, Duration::from_secs(1)), 100.0);
        assert_eq!(chunk_size(1, Duration::from_secs(10)), 64_000);
        assert_eq!(
            chunk_size(1_000_000_000, Duration::from_millis(1)),
            32_000_000
        );
    }

    #[test]
    #[ignore = "Contacts Cloudflare and transfers network data; run explicitly"]
    fn cloudflare_endpoint_smoke() {
        let cancel = AtomicBool::new(false);
        let base = "https://speed.cloudflare.com";
        assert!(latency(base, &cancel).unwrap().is_finite());
        for upload in [false, true] {
            assert!(
                throughput(
                    base,
                    upload,
                    &cancel,
                    Duration::from_millis(250),
                    Duration::from_millis(750),
                    |_| {}
                )
                .unwrap()
                    > 0.0
            );
        }
    }
}
