//! Compare actual DNS responses, without modifying network settings.
use std::{
    net::{IpAddr, SocketAddr, UdpSocket},
    sync::atomic::{AtomicU16, Ordering},
    time::{Duration, Instant},
};

const TIMEOUT: Duration = Duration::from_millis(700);
const DOMAINS: [&str; 3] = ["example.com", "www.microsoft.com", "www.cloudflare.com"];
static QUERY_ID: AtomicU16 = AtomicU16::new(1);

#[derive(Clone)]
pub struct Candidate {
    pub provider_id: String,
    pub name: String,
    pub address: IpAddr,
}

pub struct Measurement {
    pub candidate: Candidate,
    pub median_ms: Option<f64>,
    pub successful: usize,
}

fn query(id: u16, domain: &str) -> Vec<u8> {
    let mut bytes = Vec::from(id.to_be_bytes());
    bytes.extend_from_slice(&[1, 0, 0, 1, 0, 0, 0, 0, 0, 0]);
    for label in domain.split('.') {
        bytes.push(label.len() as u8);
        bytes.extend_from_slice(label.as_bytes());
    }
    bytes.extend_from_slice(&[0, 0, 1, 0, 1]); // A, IN
    bytes
}

fn valid_response(request: &[u8], response: &[u8]) -> bool {
    // Check transaction, standard response, no truncation/error, a matching question,
    // and at least one answer. Reject unrelated, empty, and SERVFAIL responses.
    if !(response.len() >= request.len() + 12
        && response[..2] == request[..2]
        && response[2] & 0xfa == 0x80
        && response[3] & 0x0f == 0
        && response[4..6] == [0, 1]
        && u16::from_be_bytes([response[6], response[7]]) > 0
        && response[12..request.len()] == request[12..])
    {
        return false;
    }
    let mut offset = request.len();
    let mut has_address = false;
    for _ in 0..u16::from_be_bytes([response[6], response[7]]) {
        loop {
            let Some(&length) = response.get(offset) else {
                return false;
            };
            offset += 1;
            if length == 0 {
                break;
            }
            if length & 0xc0 == 0xc0 {
                let Some(&low) = response.get(offset) else {
                    return false;
                };
                let pointer = (((length & 0x3f) as usize) << 8) | low as usize;
                if pointer >= response.len() {
                    return false;
                }
                offset += 1;
                break;
            }
            if length > 63 {
                return false;
            }
            offset += length as usize;
        }
        let Some(header) = response.get(offset..offset + 10) else {
            return false;
        };
        let record_type = u16::from_be_bytes([header[0], header[1]]);
        let class = u16::from_be_bytes([header[2], header[3]]);
        let length = u16::from_be_bytes([header[8], header[9]]) as usize;
        offset += 10;
        if response.get(offset..offset + length).is_none() {
            return false;
        }
        has_address |= record_type == 1 && class == 1 && length == 4;
        offset += length;
    }
    has_address
}

fn probe(address: IpAddr, domain: &str) -> Result<f64, std::io::Error> {
    probe_at(SocketAddr::new(address, 53), domain)
}

fn probe_at(address: SocketAddr, domain: &str) -> Result<f64, std::io::Error> {
    probe_at_with_timeout(address, domain, TIMEOUT)
}

fn probe_at_with_timeout(
    address: SocketAddr,
    domain: &str,
    timeout: Duration,
) -> Result<f64, std::io::Error> {
    let socket = UdpSocket::bind(if address.is_ipv4() {
        "0.0.0.0:0"
    } else {
        "[::]:0"
    })?;
    socket.connect(address)?;
    socket.set_write_timeout(Some(timeout))?;
    let request = query(QUERY_ID.fetch_add(1, Ordering::Relaxed), domain);
    let started = Instant::now();
    socket.send(&request)?;
    let mut response = [0u8; 4096];
    loop {
        let remaining = timeout
            .checked_sub(started.elapsed())
            .filter(|d| !d.is_zero())
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::TimedOut, "DNS response timed out")
            })?;
        socket.set_read_timeout(Some(remaining))?;
        let count = socket.recv(&mut response)?;
        if valid_response(&request, &response[..count]) {
            return Ok(started.elapsed().as_secs_f64() * 1000.0);
        }
    }
}

/// Check real name resolution, including DNS proxies on local routers.
pub fn resolver_latency(address: IpAddr) -> Result<u128, String> {
    for domain in DOMAINS.iter().take(2) {
        if let Ok(ms) =
            probe_at_with_timeout(SocketAddr::new(address, 53), domain, Duration::from_secs(2))
        {
            return Ok(ms.round().max(1.0) as u128);
        }
    }
    Err(format!(
        "Could not verify DNS responses from {address}. UDP DNS may be unavailable or blocked."
    ))
}

fn measure(candidate: &Candidate) -> Measurement {
    let mut times = DOMAINS
        .into_iter()
        .filter_map(|domain| probe(candidate.address, domain).ok())
        .collect::<Vec<_>>();
    times.sort_by(f64::total_cmp);
    let successful = times.len();
    let median_ms = if successful >= 2 {
        Some((times[(successful - 1) / 2] + times[successful / 2]) / 2.0)
    } else {
        None
    };
    Measurement {
        candidate: candidate.clone(),
        median_ms,
        successful,
    }
}

pub fn benchmark(candidates: Vec<Candidate>) -> Vec<Measurement> {
    let mut measurements = Vec::with_capacity(candidates.len());
    // Bound concurrent sockets/threads and avoid blocking the UI event loop.
    for chunk in candidates.chunks(4) {
        std::thread::scope(|scope| {
            let handles = chunk
                .iter()
                .map(|candidate| scope.spawn(move || measure(candidate)))
                .collect::<Vec<_>>();
            for handle in handles {
                if let Ok(result) = handle.join() {
                    measurements.push(result);
                }
            }
        });
    }
    measurements.sort_by(|a, b| {
        b.successful.cmp(&a.successful).then_with(|| {
            a.median_ms
                .unwrap_or(f64::INFINITY)
                .total_cmp(&b.median_ms.unwrap_or(f64::INFINITY))
        })
    });
    measurements
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ignores_unrelated_packet_before_accepting_dns_answer() {
        let server = UdpSocket::bind("127.0.0.1:0").unwrap();
        server
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let address = server.local_addr().unwrap();
        let worker = std::thread::spawn(move || {
            let mut request = [0; 512];
            let (length, peer) = server.recv_from(&mut request).unwrap();
            let mut answer = request[..length].to_vec();
            answer[2] = 0x81;
            answer[3] = 0x80;
            answer[7] = 1;
            answer.extend_from_slice(&[0xc0, 12, 0, 1, 0, 1, 0, 0, 0, 60, 0, 4, 93, 184, 215, 14]);
            answer[0] ^= 1;
            server.send_to(&answer, peer).unwrap();
            answer[0] ^= 1;
            server.send_to(&answer, peer).unwrap();
        });
        assert!(probe_at(address, "example.com").unwrap() < 700.0);
        worker.join().unwrap();
    }
    #[test]
    fn rejects_mismatched_truncated_and_failed_responses() {
        let request = query(42, "example.com");
        let mut response = request.clone();
        response[2] = 0x81;
        response[3] = 0x80;
        response[7] = 1;
        response.extend_from_slice(&[0xc0, 12, 0, 1, 0, 1, 0, 0, 0, 60, 0, 4, 93, 184, 215, 14]);
        assert!(valid_response(&request, &response));
        assert!(!valid_response(&request, &response[..response.len() - 1]));
        assert!(!valid_response(&request, &response[..10]));
        response[0] ^= 1;
        assert!(!valid_response(&request, &response));
        response[0] ^= 1;
        response[2] |= 2;
        assert!(!valid_response(&request, &response));
        response[2] &= !2;
        response[3] |= 2;
        assert!(!valid_response(&request, &response));
        response[3] &= !2;
        response[13] ^= 1;
        assert!(!valid_response(&request, &response));
    }
}
