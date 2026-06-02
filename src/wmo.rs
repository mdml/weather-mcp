//! WMO 4677 weather-code → label table (tool-specs Appendix B).
//!
//! The server owns this map so the model/app needn't carry one; `get_forecast` echoes both the
//! raw `weather_code` and the decoded `weather` label. The table is part of the contract and is
//! pinned by snapshot. This is real, complete, deterministic data — not a Phase 3 stub.

/// Decode a WMO 4677 weather code to its human label. Unknown codes return `"Unknown"`.
pub fn decode(code: u8) -> &'static str {
    match code {
        0 => "Clear sky",
        1 => "Mainly clear",
        2 => "Partly cloudy",
        3 => "Overcast",
        45 => "Fog",
        48 => "Depositing rime fog",
        51 => "Light drizzle",
        53 => "Moderate drizzle",
        55 => "Dense drizzle",
        56 => "Light freezing drizzle",
        57 => "Dense freezing drizzle",
        61 => "Slight rain",
        63 => "Moderate rain",
        65 => "Heavy rain",
        66 => "Light freezing rain",
        67 => "Heavy freezing rain",
        71 => "Slight snow",
        73 => "Moderate snow",
        75 => "Heavy snow",
        77 => "Snow grains",
        80 => "Slight rain showers",
        81 => "Moderate rain showers",
        82 => "Violent rain showers",
        85 => "Slight snow showers",
        86 => "Heavy snow showers",
        95 => "Thunderstorm",
        96 => "Thunderstorm with slight hail",
        99 => "Thunderstorm with heavy hail",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::decode;

    #[test]
    fn decodes_the_appendix_b_anchor_codes() {
        // The abbreviated table called out explicitly in tool-specs Appendix B.
        assert_eq!(decode(0), "Clear sky");
        assert_eq!(decode(1), "Mainly clear");
        assert_eq!(decode(2), "Partly cloudy");
        assert_eq!(decode(3), "Overcast");
        assert_eq!(decode(45), "Fog");
        assert_eq!(decode(48), "Depositing rime fog");
        assert_eq!(decode(61), "Slight rain");
        assert_eq!(decode(63), "Moderate rain");
        assert_eq!(decode(65), "Heavy rain");
        assert_eq!(decode(71), "Slight snow");
        assert_eq!(decode(75), "Heavy snow");
        assert_eq!(decode(80), "Slight rain showers");
        assert_eq!(decode(95), "Thunderstorm");
        assert_eq!(decode(99), "Thunderstorm with heavy hail");
    }

    #[test]
    fn unknown_codes_decode_to_unknown() {
        assert_eq!(decode(4), "Unknown");
        assert_eq!(decode(200), "Unknown");
    }
}
