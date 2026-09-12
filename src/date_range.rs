//! A `--since`/`--until` date window used to scope every report to a fiscal period. See
//! `docs/DATE_RANGE.md` for the full design and rationale.
//!
//! The window is applied to a journal's transactions *before* a [`crate::ledger::Ledger`] is
//! built from them (see `main.rs`'s `run_*` functions) — reports themselves stay date-unaware,
//! pure functions of whichever transactions the `Ledger` was actually given.

use crate::model::Transaction;
use chrono::NaiveDate;

/// An inclusive-start, exclusive-end date window: a transaction is in range when
/// `since <= date < until`. Either bound may be absent, meaning "no lower/upper bound" — the
/// default `DateRange` (`since: None, until: None`) matches every date, so filtering with it is
/// a no-op. This mirrors hledger's `-b/--begin` (inclusive) and `-e/--end` (exclusive)
/// semantics, under the (arguably clearer) names `--since`/`--until`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DateRange {
    /// Earliest date included, inclusive. `None` means no lower bound.
    pub since: Option<NaiveDate>,
    /// Earliest date *excluded* from the upper end — i.e. the range includes dates strictly
    /// before this one. `None` means no upper bound.
    pub until: Option<NaiveDate>,
}

impl DateRange {
    /// Builds a date range from optional bounds.
    pub fn new(since: Option<NaiveDate>, until: Option<NaiveDate>) -> Self {
        DateRange { since, until }
    }

    /// True if neither bound is set — filtering with this range would keep everything.
    pub fn is_unbounded(&self) -> bool {
        self.since.is_none() && self.until.is_none()
    }

    /// True if `date` falls within this range (`since <= date < until`, treating an absent
    /// bound as unconstrained on that side).
    pub fn contains(&self, date: NaiveDate) -> bool {
        let after_since = self.since.is_none_or(|s| date >= s);
        let before_until = self.until.is_none_or(|u| date < u);
        after_since && before_until
    }

    /// Keeps only the transactions whose date falls within this range. A fully unbounded range
    /// returns `transactions` unchanged (no allocation, no reordering).
    pub fn filter(&self, transactions: Vec<Transaction>) -> Vec<Transaction> {
        if self.is_unbounded() {
            return transactions;
        }
        transactions.into_iter().filter(|t| self.contains(t.date)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn unbounded_range_contains_everything() {
        let range = DateRange::default();
        assert!(range.is_unbounded());
        assert!(range.contains(date("2024-01-01")));
        assert!(range.contains(date("1900-01-01")));
    }

    #[test]
    fn since_is_inclusive_and_until_is_exclusive() {
        let range = DateRange::new(Some(date("2024-02-01")), Some(date("2024-03-01")));
        assert!(!range.contains(date("2024-01-31")));
        assert!(range.contains(date("2024-02-01"))); // since: inclusive
        assert!(range.contains(date("2024-02-29")));
        assert!(!range.contains(date("2024-03-01"))); // until: exclusive
    }

    #[test]
    fn one_sided_ranges_only_constrain_their_side() {
        let since_only = DateRange::new(Some(date("2024-02-01")), None);
        assert!(!since_only.contains(date("2024-01-01")));
        assert!(since_only.contains(date("2099-01-01")));

        let until_only = DateRange::new(None, Some(date("2024-02-01")));
        assert!(until_only.contains(date("1900-01-01")));
        assert!(!until_only.contains(date("2024-02-01")));
    }
}
