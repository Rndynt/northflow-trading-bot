//! Small CLI output helpers for research-mode presentation.

use std::fmt::Display;

pub fn section(title: &str) {
    println!("{title}");
    println!("{}", "=".repeat(title.chars().count().max(1)));
}

pub fn subsection(title: &str) {
    println!("{title}");
    println!("{}", "-".repeat(title.chars().count().max(1)));
}

pub fn key_value(label: &str, value: impl Display) {
    println!("{label:<24}: {value}");
}

pub fn bullet(value: impl Display) {
    println!("  - {value}");
}

pub fn numbered(index: usize, value: impl Display) {
    println!("  {index}. {value}");
}

pub fn blank() {
    println!();
}

pub fn format_int(value: usize) -> String {
    let s = value.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    let first_group = s.len() % 3;
    for (idx, ch) in s.chars().enumerate() {
        if idx > 0 && (idx == first_group || (idx > first_group && (idx - first_group) % 3 == 0)) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

pub fn format_f64(value: f64, decimals: usize) -> String {
    format!("{value:.decimals$}")
}

pub fn progress_line(
    current: usize,
    total: usize,
    trades: usize,
    elapsed: &str,
    width: usize,
) -> String {
    let percent = if total == 0 {
        100.0
    } else {
        current as f64 / total as f64 * 100.0
    };
    let filled = if total == 0 {
        width
    } else {
        ((percent / 100.0) * width as f64).round() as usize
    }
    .min(width);
    format!(
        "[{}{}] {:>5.1}% {}/{} candles | trades: {} | elapsed: {}",
        "#".repeat(filled),
        "-".repeat(width.saturating_sub(filled)),
        percent,
        format_int(current),
        format_int(total),
        format_int(trades),
        elapsed
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_large_ints_with_commas() {
        assert_eq!(format_int(3_156_480), "3,156,480");
        assert_eq!(format_int(0), "0");
        assert_eq!(format_int(999), "999");
        assert_eq!(format_int(1_000), "1,000");
    }

    #[test]
    fn progress_formatter_renders_key_percentages() {
        assert!(progress_line(0, 100, 0, "00:00", 10).contains("  0.0%"));
        assert!(progress_line(50, 100, 3, "00:01", 10).contains(" 50.0%"));
        assert!(progress_line(100, 100, 9, "00:02", 10).contains("100.0%"));
    }

    #[test]
    fn helpers_accept_empty_values() {
        section("");
        subsection("");
        key_value("", "");
        bullet("");
        numbered(1, "");
        blank();
    }
}
