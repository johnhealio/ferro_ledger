//! Library-level tests for date-range filtering: [`ferro_ledger::date_range::DateRange`] applied
//! to a loaded journal's transactions before a report is built. CLI-level wiring (the
//! `--since`/`--until` flags themselves) is covered separately in `cli_date_range_tests.rs`.

use ferro_ledger::date_range::DateRange;
use ferro_ledger::journal::load_journal;
use ferro_ledger::ledger::Ledger;
use ferro_ledger::reports::trial_balance;
use rust_decimal::Decimal;

fn fixture() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/date_range.journal")
}

fn date(s: &str) -> chrono::NaiveDate {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
}

#[test]
fn unbounded_range_includes_every_transaction() {
    let transactions = load_journal(fixture()).unwrap();
    let range = DateRange::default();
    assert_eq!(range.filter(transactions).len(), 3);
}

#[test]
fn since_excludes_earlier_transactions() {
    let transactions = load_journal(fixture()).unwrap();
    let range = DateRange::new(Some(date("2024-02-15")), None);
    let filtered = range.filter(transactions);
    assert_eq!(filtered.len(), 2);
    assert_eq!(filtered[0].description, "February consulting revenue");
}

#[test]
fn until_is_exclusive_of_the_boundary_date() {
    let transactions = load_journal(fixture()).unwrap();
    let range = DateRange::new(None, Some(date("2024-02-15")));
    let filtered = range.filter(transactions);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].description, "Opening balances");
}

#[test]
fn a_narrow_window_changes_the_trial_balance_totals() {
    let transactions = load_journal(fixture()).unwrap();
    let range = DateRange::new(Some(date("2024-02-01")), Some(date("2024-03-01")));
    let filtered = range.filter(transactions);
    assert_eq!(filtered.len(), 1); // only the February transaction

    let ledger = Ledger::from_transactions(filtered);
    let report = trial_balance::build(&ledger);
    assert!(report.is_balanced());
    let section = &report.sections[0];
    assert_eq!(section.total_debit, Decimal::new(50000, 2)); // just the 500.00 consulting entry
}
