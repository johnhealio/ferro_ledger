use ferro_ledger::ledger::Ledger;
use ferro_ledger::model::Account;
use ferro_ledger::parser::parse_str;
use ferro_ledger::reports::clearing;

fn ledger_from(src: &str) -> Ledger {
    let parsed = parse_str(src, "test.journal").expect("should parse");
    Ledger::from_transactions(parsed.transactions)
}

#[test]
fn tag_matched_group_clears_when_it_nets_to_zero() {
    let ledger = ledger_from(
        "2024-01-05 * Invoice recorded\n\
         \x20\x20\x20\x20Assets:Clearing:Payments   500.00 USD  ; match:INV-1\n\
         \x20\x20\x20\x20Income:Sales              -500.00 USD\n\
         \n\
         2024-01-09 * Bank settlement\n\
         \x20\x20\x20\x20Assets:Bank:Checking        500.00 USD\n\
         \x20\x20\x20\x20Assets:Clearing:Payments  -500.00 USD  ; match:INV-1\n",
    );

    let reports = clearing::analyze(&ledger, &[Account::new("Assets:Clearing:Payments")]);
    assert_eq!(reports.len(), 1);
    let report = &reports[0];
    assert_eq!(report.cleared_count(), 1);
    assert_eq!(report.outstanding_count(), 0);
    assert!(report.groups[0].cleared);
}

#[test]
fn unmatched_posting_is_outstanding() {
    let ledger = ledger_from(
        "2024-01-12 * Invoice recorded, not yet settled\n\
         \x20\x20\x20\x20Assets:Clearing:Payments   320.00 USD  ; match:INV-2\n\
         \x20\x20\x20\x20Income:Sales              -320.00 USD\n",
    );

    let reports = clearing::analyze(&ledger, &[Account::new("Assets:Clearing:Payments")]);
    let report = &reports[0];
    assert_eq!(report.cleared_count(), 0);
    assert_eq!(report.outstanding_count(), 1);
    assert_eq!(report.groups[0].net, rust_decimal::Decimal::new(32000, 2));
}

#[test]
fn fallback_matching_pairs_postings_of_equal_absolute_amount_with_no_tag() {
    let ledger = ledger_from(
        "2024-02-01 * Leg one\n\
         \x20\x20\x20\x20Assets:Clearing:Suspense   100.00 USD\n\
         \x20\x20\x20\x20Income:Misc               -100.00 USD\n\
         \n\
         2024-02-02 * Leg two\n\
         \x20\x20\x20\x20Assets:Bank:Checking        100.00 USD\n\
         \x20\x20\x20\x20Assets:Clearing:Suspense  -100.00 USD\n",
    );

    let reports = clearing::analyze(&ledger, &[Account::new("Assets:Clearing:Suspense")]);
    let report = &reports[0];
    assert_eq!(report.cleared_count(), 1);
    assert_eq!(report.groups[0].members.len(), 2);
}

#[test]
fn subtree_rollup_includes_descendant_clearing_accounts() {
    let ledger = ledger_from(
        "2024-03-01 * Sub-account leg one\n\
         \x20\x20\x20\x20Assets:Clearing:Payroll:US   400.00 USD  ; match:P1\n\
         \x20\x20\x20\x20Expenses:Payroll            -400.00 USD\n\
         \n\
         2024-03-02 * Sub-account leg two\n\
         \x20\x20\x20\x20Assets:Bank:Checking          400.00 USD\n\
         \x20\x20\x20\x20Assets:Clearing:Payroll:US  -400.00 USD  ; match:P1\n",
    );

    let reports = clearing::analyze(&ledger, &[Account::new("Assets:Clearing:Payroll")]);
    let report = &reports[0];
    assert_eq!(report.cleared_count(), 1);
}
