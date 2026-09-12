//! End-to-end tests of the `--since`/`--until` flags through the actual `ferro_ledger` binary
//! (not just the library-level filtering logic — see `date_range_tests.rs` for that), to catch
//! any wiring mistake in `cli.rs`'s `#[command(flatten)]` or `main.rs`'s flag handling that a
//! purely library-level test wouldn't see.

use std::process::Command;

fn fixture_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/date_range.journal")
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ferro_ledger"))
        .args(args)
        .output()
        .expect("failed to run ferro_ledger binary")
}

#[test]
fn balance_without_date_flags_includes_every_transaction() {
    let output = run(&["balance", fixture_path().to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Expenses:Rent"));
    assert!(!stdout.contains("Period:"));
}

#[test]
fn balance_with_since_excludes_earlier_transactions() {
    let output = run(&["balance", fixture_path().to_str().unwrap(), "--since", "2024-02-15"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Period: 2024-02-15 onward"));
    assert!(stdout.contains("Expenses:Rent"));
    assert!(!stdout.contains("Equity:OpeningBalance"));
}

#[test]
fn balance_with_until_excludes_later_transactions() {
    let output = run(&["balance", fixture_path().to_str().unwrap(), "--until", "2024-02-15"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Period: up to 2024-02-15 (exclusive)"));
    assert!(stdout.contains("Equity:OpeningBalance"));
    assert!(!stdout.contains("Income:Consulting"));
    assert!(!stdout.contains("Expenses:Rent"));
}

#[test]
fn income_statement_respects_date_range() {
    let output = run(&[
        "income-statement",
        fixture_path().to_str().unwrap(),
        "--since",
        "2024-02-01",
        "--until",
        "2024-03-01",
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Income:Consulting"));
    assert!(!stdout.contains("Expenses:Rent"));
}

#[test]
fn invalid_date_flag_fails_with_a_clear_error() {
    let output = run(&["balance", fixture_path().to_str().unwrap(), "--since", "not-a-date"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid --since date"));
}

#[test]
fn check_subcommand_has_no_date_range_flags() {
    let output = run(&["check", fixture_path().to_str().unwrap(), "--since", "2024-02-01"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unexpected argument"));
}
