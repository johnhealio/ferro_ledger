use ferro_ledger::ledger::Ledger;
use ferro_ledger::parser::parse_str;
use ferro_ledger::reports::balance_sheet;
use rust_decimal::Decimal;

fn ledger_from(src: &str) -> Ledger {
    let parsed = parse_str(src, "test.journal").expect("should parse");
    Ledger::from_transactions(parsed.transactions)
}

#[test]
fn balances_with_no_income_or_expense_activity() {
    let ledger = ledger_from(
        "2024-01-01 * Opening balances\n\
         \x20\x20\x20\x20Assets:Bank:Checking   1000.00 USD\n\
         \x20\x20\x20\x20Equity:OpeningBalance -1000.00 USD\n",
    );
    let report = balance_sheet::build(&ledger);
    assert_eq!(report.sheets.len(), 1);
    let sheet = &report.sheets[0];
    assert!(sheet.is_balanced());
    assert_eq!(sheet.assets.total, Decimal::new(100000, 2));
    assert_eq!(sheet.liabilities.total, Decimal::ZERO);
    assert_eq!(sheet.equity.total, Decimal::new(100000, 2));
    assert!(report.unclassified_accounts.is_empty());
}

#[test]
fn folds_unclosed_net_income_into_equity_so_it_balances() {
    let ledger = ledger_from(
        "2024-01-01 * Opening balances\n\
         \x20\x20\x20\x20Assets:Bank:Checking   1000.00 USD\n\
         \x20\x20\x20\x20Equity:OpeningBalance -1000.00 USD\n\
         \n\
         2024-01-05 * Consulting revenue\n\
         \x20\x20\x20\x20Assets:Bank:Checking    500.00 USD\n\
         \x20\x20\x20\x20Income:Consulting      -500.00 USD\n\
         \n\
         2024-01-10 * Office rent\n\
         \x20\x20\x20\x20Expenses:Rent           200.00 USD\n\
         \x20\x20\x20\x20Assets:Bank:Checking\n",
    );
    let report = balance_sheet::build(&ledger);
    let sheet = &report.sheets[0];
    assert!(sheet.is_balanced());

    // Net income = revenue (500) - expenses (200) = 300.
    let net_income_row = sheet
        .equity
        .rows
        .iter()
        .find(|r| r.account == "Net Income (unclosed)")
        .expect("net income row present");
    assert_eq!(net_income_row.amount, Decimal::new(30000, 2));

    assert_eq!(sheet.assets.total, Decimal::new(130000, 2)); // 1000 + 500 - 200
    assert_eq!(sheet.liabilities.total, Decimal::ZERO);
    assert_eq!(sheet.equity.total, Decimal::new(130000, 2)); // 1000 opening + 300 net income
}

#[test]
fn liabilities_and_equity_display_positive_for_their_normal_balance() {
    let ledger = ledger_from(
        "2024-01-01 * Opening balances\n\
         \x20\x20\x20\x20Assets:Bank:Checking       1000.00 USD\n\
         \x20\x20\x20\x20Liabilities:CreditCard      -300.00 USD\n\
         \x20\x20\x20\x20Equity:OpeningBalance       -700.00 USD\n",
    );
    let report = balance_sheet::build(&ledger);
    let sheet = &report.sheets[0];
    assert!(sheet.is_balanced());
    assert_eq!(sheet.liabilities.rows[0].amount, Decimal::new(30000, 2));
    assert_eq!(sheet.equity.rows[0].amount, Decimal::new(70000, 2));
}

#[test]
fn accounts_with_unrecognized_top_level_segment_are_excluded_and_listed() {
    let ledger = ledger_from(
        "2024-01-01 * Odd account naming\n\
         \x20\x20\x20\x20Misc:Whatever   50.00 USD\n\
         \x20\x20\x20\x20Equity:Opening  -50.00 USD\n",
    );
    let report = balance_sheet::build(&ledger);
    assert_eq!(report.unclassified_accounts.len(), 1);
    assert_eq!(report.unclassified_accounts[0].0, "Misc:Whatever");
    // Still balances: Equity got its -(-50.00) = 50.00, Misc:Whatever isn't on any sheet.
    let sheet = &report.sheets[0];
    assert!(sheet.assets.rows.is_empty());
    assert_eq!(sheet.equity.total, Decimal::new(5000, 2));
}
