use ferro_ledger::ledger::Ledger;
use ferro_ledger::parser::parse_str;
use ferro_ledger::reports::{balance_sheet, income_statement};
use rust_decimal::Decimal;

fn ledger_from(src: &str) -> Ledger {
    let parsed = parse_str(src, "test.journal").expect("should parse");
    Ledger::from_transactions(parsed.transactions)
}

#[test]
fn revenue_and_expenses_display_positive_with_correct_net_income() {
    let ledger = ledger_from(
        "2024-01-05 * Consulting revenue\n\
         \x20\x20\x20\x20Assets:Bank:Checking    500.00 USD\n\
         \x20\x20\x20\x20Income:Consulting      -500.00 USD\n\
         \n\
         2024-01-10 * Office rent\n\
         \x20\x20\x20\x20Expenses:Rent           200.00 USD\n\
         \x20\x20\x20\x20Assets:Bank:Checking   -200.00 USD\n",
    );
    let report = income_statement::build(&ledger);
    assert_eq!(report.statements.len(), 1);
    let statement = &report.statements[0];

    assert_eq!(statement.revenue.rows.len(), 1);
    assert_eq!(statement.revenue.rows[0].amount, Decimal::new(50000, 2));
    assert_eq!(statement.expenses.rows[0].amount, Decimal::new(20000, 2));
    assert_eq!(statement.net_income(), Decimal::new(30000, 2));
    assert!(report.unclassified_accounts.is_empty());
}

#[test]
fn asset_liability_equity_accounts_are_out_of_scope_not_flagged() {
    let ledger = ledger_from(
        "2024-01-01 * Opening balances\n\
         \x20\x20\x20\x20Assets:Bank:Checking   1000.00 USD\n\
         \x20\x20\x20\x20Equity:OpeningBalance -1000.00 USD\n",
    );
    let report = income_statement::build(&ledger);
    // No revenue/expense activity at all, so no statement is produced for any commodity, and
    // the Asset/Equity accounts are correctly out of scope rather than "unclassified."
    assert!(report.statements.is_empty());
    assert!(report.unclassified_accounts.is_empty());
}

#[test]
fn unrecognized_top_level_segment_is_listed_but_not_asset_liability_equity() {
    let ledger = ledger_from(
        "2024-01-01 * Mixed\n\
         \x20\x20\x20\x20Misc:Whatever          50.00 USD\n\
         \x20\x20\x20\x20Assets:Bank:Checking  -50.00 USD\n",
    );
    let report = income_statement::build(&ledger);
    assert_eq!(report.unclassified_accounts.len(), 1);
    assert_eq!(report.unclassified_accounts[0].0, "Misc:Whatever");
    assert!(report.statements.is_empty());
}

#[test]
fn net_income_matches_balance_sheets_folded_net_income_row() {
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
         \x20\x20\x20\x20Assets:Bank:Checking   -200.00 USD\n",
    );

    let income_report = income_statement::build(&ledger);
    let net_income = income_report.statements[0].net_income();

    let bs_report = balance_sheet::build(&ledger);
    let bs_sheet = &bs_report.sheets[0];
    let folded_row = bs_sheet
        .equity
        .rows
        .iter()
        .find(|r| r.account == "Net Income (unclosed)")
        .expect("balance sheet should fold in a net income row");

    assert_eq!(net_income, folded_row.amount);
}
