//! Integration tests for `journal::load_journal`, which is where file I/O and `include`
//! resolution happen (see `docs/ARCHITECTURE.md`). Unit-level parser edge cases (tag parsing,
//! amount lexing, balancing) live in `src/parser.rs`'s own `#[cfg(test)]` module.

use ferro_ledger::journal::{load_journal, JournalError};

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn resolves_include_directive_relative_to_including_file() {
    let transactions = load_journal(fixture("main_with_include.journal")).expect("should load");
    assert_eq!(transactions.len(), 2);
    assert_eq!(transactions[0].description, "Opening balances");
    assert_eq!(transactions[1].description, "Included transaction");
}

#[test]
fn detects_circular_include() {
    let err = load_journal(fixture("circular_a.journal")).unwrap_err();
    match err {
        JournalError::CircularInclude { .. } => {}
        other => panic!("expected CircularInclude, got {other:?}"),
    }
}

#[test]
fn reports_missing_file() {
    let err = load_journal(fixture("does_not_exist.journal")).unwrap_err();
    match err {
        JournalError::Io { .. } => {}
        other => panic!("expected Io error, got {other:?}"),
    }
}
