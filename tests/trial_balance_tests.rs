use ferro_ledger::ledger::Ledger;
use ferro_ledger::parser::parse_str;
use ferro_ledger::reports::trial_balance;
use rust_decimal::Decimal;

fn ledger_from(src: &str) -> Ledger {
    let parsed = parse_str(src, "test.journal").expect("should parse");
    Ledger::from_transactions(parsed.transactions)
}

#[test]
fn trial_balance_foots_to_equal_debits_and_credits() {
    let ledger = ledger_from(
        "2024-01-01 * Opening balances\n\
         \x20\x20\x20\x20Assets:Bank:Checking   1000.00 USD\n\
         \x20\x20\x20\x20Equity:OpeningBalance -1000.00 USD\n\
         \n\
         2024-01-05 Office supplies\n\
         \x20\x20\x20\x20Expenses:Office   45.00 USD\n\
         \x20\x20\x20\x20Assets:Bank:Checking\n",
    );

    let report = trial_balance::build(&ledger);
    assert_eq!(report.sections.len(), 1);
    let section = &report.sections[0];
    assert_eq!(section.commodity, "USD");
    assert!(section.is_balanced());
    assert_eq!(section.total_debit, Decimal::new(100000, 2));
    assert_eq!(section.total_credit, Decimal::new(100000, 2));

    let checking_row = section
        .rows
        .iter()
        .find(|r| r.account.0 == "Assets:Bank:Checking")
        .expect("checking row present");
    assert_eq!(checking_row.debit, Decimal::new(95500, 2));
    assert_eq!(checking_row.credit, Decimal::ZERO);
}

#[test]
fn zero_balance_accounts_are_omitted() {
    let ledger = ledger_from(
        "2024-01-01 * A\n    Assets:Wash   10.00 USD\n    Equity:Wash  -10.00 USD\n\n\
         2024-01-02 * B (reverses A)\n    Assets:Wash  -10.00 USD\n    Equity:Wash   10.00 USD\n",
    );
    let report = trial_balance::build(&ledger);
    // Both accounts net to zero across the two transactions, so a trial balance omits them.
    assert!(report.sections.is_empty() || report.sections.iter().all(|s| s.rows.is_empty()));
}

#[test]
fn separate_commodities_get_separate_sections() {
    let ledger = ledger_from(
        "2024-01-01 * Multi-currency\n\
         \x20\x20\x20\x20Assets:Bank:USD   100.00 USD\n\
         \x20\x20\x20\x20Equity:Opening   -100.00 USD\n\
         \n\
         2024-01-02 * Euro account\n\
         \x20\x20\x20\x20Assets:Bank:EUR    50.00 EUR\n\
         \x20\x20\x20\x20Equity:Opening    -50.00 EUR\n",
    );
    let report = trial_balance::build(&ledger);
    assert_eq!(report.sections.len(), 2);
    assert!(report.is_balanced());
    let commodities: Vec<&str> = report.sections.iter().map(|s| s.commodity.as_str()).collect();
    assert!(commodities.contains(&"USD"));
    assert!(commodities.contains(&"EUR"));
}
