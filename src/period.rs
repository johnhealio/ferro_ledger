//! `--period` shorthand: expands a period expression like `2024`, `2024-01`, or
//! `2024-01 to 2024-03` into the equivalent [`DateRange`] a user would otherwise have to spell
//! out with `--since`/`--until`. See `docs/DATE_RANGE.md`, "Period shorthand (`--period`)", for
//! the full grammar and worked examples.

use crate::date_range::DateRange;
use crate::parser::parse_date_str;
use chrono::NaiveDate;

/// Parses a period expression into the [`DateRange`] it denotes.
///
/// Grammar: `["from"] TERM ["to" TERM]`, where `TERM` is one of:
/// - `YYYY` — that whole calendar year.
/// - `YYYY-MM` (`/` or `.` separators also accepted) — that whole calendar month.
/// - `YYYY-MM-DD` (any format [`parse_date_str`] accepts) — that single day.
///
/// A single term denotes its own `[start, end)` range. A `TERM to TERM` range spans from the
/// first term's start to the second term's end — the two terms need not share a granularity
/// (`"2024-01 to 2024"` is valid, spanning from the start of January to the end of that year).
pub fn parse_period(input: &str) -> Result<DateRange, String> {
    let mut words: Vec<&str> = input.split_whitespace().collect();
    if words.first().is_some_and(|w| w.eq_ignore_ascii_case("from")) {
        words.remove(0);
    }
    if words.is_empty() {
        return Err("empty period expression".to_string());
    }

    match words.iter().position(|w| w.eq_ignore_ascii_case("to")) {
        None => {
            let (start, end) = parse_term(&words.join(" "))?;
            Ok(DateRange::new(Some(start), Some(end)))
        }
        Some(pos) => {
            let left = words[..pos].join(" ");
            let right = words[pos + 1..].join(" ");
            if left.is_empty() || right.is_empty() {
                return Err(format!("invalid period range '{}': expected '<TERM> to <TERM>'", input));
            }
            let (start, _) = parse_term(&left)?;
            let (_, end) = parse_term(&right)?;
            Ok(DateRange::new(Some(start), Some(end)))
        }
    }
}

/// Parses one period term into the `[start, end)` range it denotes.
fn parse_term(token: &str) -> Result<(NaiveDate, NaiveDate), String> {
    if let Some(date) = parse_date_str(token) {
        return Ok(day_range(date));
    }

    let parts: Vec<&str> = token.split(['-', '/', '.']).collect();
    match parts.as_slice() {
        [y] if is_year(y) => year_range(parse_year(y)?),
        [y, m] if is_year(y) => month_range(parse_year(y)?, parse_month(m, token)?),
        _ => Err(format!(
            "invalid period term '{}': expected YYYY, YYYY-MM, or YYYY-MM-DD",
            token
        )),
    }
}

fn is_year(s: &str) -> bool {
    s.len() == 4 && s.chars().all(|c| c.is_ascii_digit())
}

fn parse_year(s: &str) -> Result<i32, String> {
    s.parse().map_err(|_| format!("invalid year '{}'", s))
}

fn parse_month(s: &str, token: &str) -> Result<u32, String> {
    s.parse().map_err(|_| format!("invalid month in '{}'", token))
}

fn year_range(year: i32) -> Result<(NaiveDate, NaiveDate), String> {
    let start = NaiveDate::from_ymd_opt(year, 1, 1).ok_or_else(|| format!("year {} out of range", year))?;
    let end =
        NaiveDate::from_ymd_opt(year + 1, 1, 1).ok_or_else(|| format!("year {} out of range", year + 1))?;
    Ok((start, end))
}

fn month_range(year: i32, month: u32) -> Result<(NaiveDate, NaiveDate), String> {
    let start = NaiveDate::from_ymd_opt(year, month, 1)
        .ok_or_else(|| format!("invalid month {} in year {}", month, year))?;
    let (end_year, end_month) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    let end = NaiveDate::from_ymd_opt(end_year, end_month, 1)
        .ok_or_else(|| format!("invalid month {} in year {}", end_month, end_year))?;
    Ok((start, end))
}

fn day_range(date: NaiveDate) -> (NaiveDate, NaiveDate) {
    let end = date.succ_opt().unwrap_or(date);
    (date, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn bare_year_spans_the_whole_calendar_year() {
        let range = parse_period("2024").unwrap();
        assert_eq!(range.since, Some(date("2024-01-01")));
        assert_eq!(range.until, Some(date("2025-01-01")));
    }

    #[test]
    fn year_month_spans_the_whole_calendar_month() {
        let range = parse_period("2024-12").unwrap();
        assert_eq!(range.since, Some(date("2024-12-01")));
        assert_eq!(range.until, Some(date("2025-01-01")));
    }

    #[test]
    fn full_date_spans_a_single_day() {
        let range = parse_period("2024-03-20").unwrap();
        assert_eq!(range.since, Some(date("2024-03-20")));
        assert_eq!(range.until, Some(date("2024-03-21")));
    }

    #[test]
    fn range_spans_start_of_left_term_to_end_of_right_term() {
        let range = parse_period("2024-01 to 2024-03").unwrap();
        assert_eq!(range.since, Some(date("2024-01-01")));
        assert_eq!(range.until, Some(date("2024-04-01")));
    }

    #[test]
    fn from_keyword_is_optional_and_case_insensitive() {
        let with_from = parse_period("From 2024-01-15 TO 2024-02-20").unwrap();
        let without_from = parse_period("2024-01-15 to 2024-02-20").unwrap();
        assert_eq!(with_from.since, without_from.since);
        assert_eq!(with_from.until, without_from.until);
        assert_eq!(with_from.since, Some(date("2024-01-15")));
        assert_eq!(with_from.until, Some(date("2024-02-21"))); // exclusive: day after Feb 20
    }

    #[test]
    fn mixed_granularity_range_is_allowed() {
        let range = parse_period("2024-01 to 2024").unwrap();
        assert_eq!(range.since, Some(date("2024-01-01")));
        assert_eq!(range.until, Some(date("2025-01-01")));
    }

    #[test]
    fn rejects_garbage_terms() {
        assert!(parse_period("not a period").is_err());
        assert!(parse_period("").is_err());
        assert!(parse_period("2024 to").is_err());
        assert!(parse_period("2024-13").is_err()); // invalid month
    }
}
