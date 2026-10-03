//! Shared display policy for decimal network speeds and measured DNS latency.
pub struct SpeedDisplay {
    pub value: String,
    pub unit: &'static str,
}

pub fn speed(mbps: f64) -> SpeedDisplay {
    let mbps = if mbps.is_finite() { mbps.max(0.0) } else { 0.0 };
    let (value, unit) = if mbps < 1.0 {
        // Flooring prevents a sub-1000 reading from rounding up to "1000 Kbps".
        (((mbps * 1000.0).floor()).to_string(), "Kbps")
    } else if mbps < 1000.0 {
        (
            trim_decimals(format!("{:.2}", (mbps * 100.0).floor() / 100.0)),
            "Mbps",
        )
    } else {
        (trim_decimals(format!("{:.2}", mbps / 1000.0)), "Gbps")
    };
    SpeedDisplay { value, unit }
}

fn trim_decimals(value: String) -> String {
    value.trim_end_matches('0').trim_end_matches('.').to_owned()
}

pub fn format_speed(mbps: f64) -> String {
    let display = speed(mbps);
    format!("{} {}", display.value, display.unit)
}

pub fn response_state(milliseconds: u128) -> &'static str {
    match milliseconds {
        0..=50 => "Excellent",
        51..=100 => "Good",
        101..=250 => "Fair",
        251..=500 => "Poor",
        _ => "Slow",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adaptive_speed_crosses_decimal_boundaries_without_four_digit_kbps() {
        for (mbps, expected) in [
            (0.0, "0 Kbps"),
            (0.999, "999 Kbps"),
            (0.9999, "999 Kbps"),
            (1.0, "1 Mbps"),
            (10.792, "10.79 Mbps"),
            (0.225, "225 Kbps"),
            (999.9999, "999.99 Mbps"),
            (1000.0, "1 Gbps"),
            (1500.0, "1.5 Gbps"),
            (-1.0, "0 Kbps"),
            (f64::NAN, "0 Kbps"),
            (f64::INFINITY, "0 Kbps"),
        ] {
            assert_eq!(format_speed(mbps), expected, "reading: {mbps}");
        }
    }

    #[test]
    fn dns_response_ratings_cover_every_threshold() {
        for (latency, expected) in [
            (0, "Excellent"),
            (50, "Excellent"),
            (51, "Good"),
            (100, "Good"),
            (101, "Fair"),
            (250, "Fair"),
            (251, "Poor"),
            (500, "Poor"),
            (501, "Slow"),
            (1222, "Slow"),
        ] {
            assert_eq!(response_state(latency), expected);
        }
    }
}
