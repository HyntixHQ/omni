use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snippet {
    pub name: String,
    pub content: String,
    #[serde(default)]
    pub shortcut: Option<String>,
}

impl Snippet {
    pub fn preview(&self, max_len: usize) -> String {
        let cleaned: String = self
            .content
            .chars()
            .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
            .collect();
        let trimmed = cleaned.trim();
        if trimmed.chars().count() <= max_len {
            trimmed.to_string()
        } else {
            let mut out: String = trimmed.chars().take(max_len.saturating_sub(1)).collect();
            out.push('…');
            out
        }
    }
}

pub fn expand_placeholders(content: &str, clipboard_text: Option<&str>) -> String {
    let mut out = String::with_capacity(content.len());
    let bytes = content.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            if let Some(end_rel) = find_placeholder_end(&content[i..]) {
                let placeholder = &content[i + 1..i + end_rel];
                let expanded = resolve_placeholder(placeholder, clipboard_text);
                out.push_str(&expanded);
                i += end_rel + 1;
                continue;
            }
        }
        out.push(content[i..].chars().next().unwrap_or(' '));
        i += content[i..].chars().next().map_or(1, char::len_utf8);
    }
    out
}

fn find_placeholder_end(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    if bytes.is_empty() || bytes[0] != b'{' {
        return None;
    }
    let mut depth = 1;
    for (idx, b) in bytes.iter().enumerate().skip(1) {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(idx);
                }
            }
            _ => {}
        }
    }
    None
}

fn resolve_placeholder(placeholder: &str, clipboard_text: Option<&str>) -> String {
    if let Some(fmt) = placeholder.strip_prefix("date:") {
        return format_date(fmt);
    }
    if let Some(fmt) = placeholder.strip_prefix("time:") {
        return format_date(fmt);
    }
    match placeholder {
        "date" => format_date("%Y-%m-%d"),
        "time" => format_date("%H:%M"),
        "datetime" => format_date("%Y-%m-%d %H:%M"),
        "year" => format_date("%Y"),
        "month" => format_date("%m"),
        "day" => format_date("%d"),
        "hour" => format_date("%H"),
        "minute" => format_date("%M"),
        "second" => format_date("%S"),
        "timestamp" => format!("{}", unix_timestamp()),
        "clipboard" | "clip" => clipboard_text.unwrap_or("").to_string(),
        other => format!("{{{}}}", other),
    }
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn format_date(fmt: &str) -> String {
    let secs = unix_timestamp();
    let days = (secs / 86_400) as i64;
    let time_of_day = (secs % 86_400) as u32;
    let hour = time_of_day / 3600;
    let minute = (time_of_day % 3600) / 60;
    let second = time_of_day % 60;
    let (year, month, day) = days_to_ymd(days);
    let mut out = String::with_capacity(fmt.len());
    let mut chars = fmt.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            if let Some(&next) = chars.peek() {
                chars.next();
                match next {
                    'Y' => out.push_str(&format!("{:04}", year)),
                    'y' => out.push_str(&format!("{:02}", year % 100)),
                    'm' => out.push_str(&format!("{:02}", month)),
                    'd' => out.push_str(&format!("{:02}", day)),
                    'H' => out.push_str(&format!("{:02}", hour)),
                    'M' => out.push_str(&format!("{:02}", minute)),
                    'S' => out.push_str(&format!("{:02}", second)),
                    'h' => out.push_str(&format!("{:02}", ((hour + 11) % 12) + 1)),
                    'p' => out.push_str(if hour < 12 { "AM" } else { "PM" }),
                    'A' => out.push_str(if hour < 12 { "AM" } else { "PM" }),
                    'a' => out.push_str(if hour < 12 { "am" } else { "pm" }),
                    '%' => out.push('%'),
                    _ => {
                        out.push('%');
                        out.push(next);
                    }
                }
            } else {
                out.push('%');
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn days_to_ymd(mut days: i64) -> (i32, u32, u32) {
    days += 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let doe = (days - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i32 + (era * 400) as i32;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    (year, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_date() {
        let s = expand_placeholders("Today is {date}", None);
        assert!(s.starts_with("Today is 20"));
    }

    #[test]
    fn expands_time() {
        let s = expand_placeholders("Now {time}", None);
        assert!(s.starts_with("Now ") && s.len() > 4);
    }

    #[test]
    fn expands_custom_format() {
        let s = expand_placeholders("{date:%Y/%m}", None);
        assert!(s.contains('/'));
    }

    #[test]
    fn expands_clipboard() {
        let s = expand_placeholders("cb={clipboard}", Some("hello"));
        assert_eq!(s, "cb=hello");
    }

    #[test]
    fn unknown_placeholder_preserved() {
        let s = expand_placeholders("a {foo} b", None);
        assert_eq!(s, "a {foo} b");
    }

    #[test]
    fn no_placeholders() {
        let s = expand_placeholders("hello world", None);
        assert_eq!(s, "hello world");
    }
}
