//! Income statement (profit & loss) report: Revenue and Expenses across the whole journal (no
//! date-range filtering in v1 — see `docs/INCOME_STATEMENT.md`), ending in a net income/loss
//! bottom line. That bottom line is computed the same way the balance sheet computes its folded
//! "Net Income (unclosed)" Equity row (`docs/BALANCE_SHEET.md`) — the two always agree, and
//! `tests/income_statement_tests.rs` checks exactly that.

use crate::account_types::{classify, AccountType};
use crate::ledger::Ledger;
use crate::model::Account;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// One account's row in a Revenue or Expenses section. `amount` is already sign-flipped so a
/// normal balance for that section reads as a positive number (see [`build`]).
pub struct IncomeStatementRow {
    /// The account this row reports on.
    pub account: Account,
    /// The display amount: positive for a normal balance in this section.
    pub amount: Decimal,
}

/// One section (Revenue or Expenses) of an income statement for a single commodity.
pub struct IncomeStatementSection {
    /// Rows in account order.
    pub rows: Vec<IncomeStatementRow>,
    /// Sum of every row's `amount`.
    pub total: Decimal,
}

/// An income statement for one commodity: Revenue less Expenses, ending in [`net_income`].
///
/// [`net_income`]: IncomeStatement::net_income
pub struct IncomeStatement {
    /// The commodity/currency symbol this statement reports on.
    pub commodity: String,
    /// Accounts classified as [`AccountType::Revenue`], sign-flipped to display positive.
    pub revenue: IncomeStatementSection,
    /// Accounts classified as [`AccountType::Expense`].
    pub expenses: IncomeStatementSection,
}

impl IncomeStatement {
    /// Revenue minus Expenses. Negative means a net loss for the period covered (the whole
    /// journal, in v1 — there's no date-range filtering yet).
    pub fn net_income(&self) -> Decimal {
        self.revenue.total - self.expenses.total
    }
}

/// The full report: one [`IncomeStatement`] per commodity, plus any accounts the classifier
/// couldn't place as Revenue or Expense (see [`crate::account_types::classify`]) — note this
/// deliberately does *not* include Asset/Liability/Equity accounts, which are simply outside an
/// income statement's scope, not an anomaly; only a truly unrecognized top-level segment lands
/// here, same criterion the balance sheet report uses.
pub struct IncomeStatementReport {
    /// One statement per commodity with any nonzero Revenue/Expense balance.
    pub statements: Vec<IncomeStatement>,
    /// Accounts with a nonzero balance whose top-level segment didn't match a recognized
    /// account type at all, sorted and de-duplicated.
    pub unclassified_accounts: Vec<Account>,
}

#[derive(Default)]
struct Accumulator {
    revenue: Vec<(Account, Decimal)>,
    expenses: Vec<(Account, Decimal)>,
}

/// Builds an income statement from a ledger's leaf-account balances. Accounts are classified by
/// [`classify`]; Asset/Liability/Equity accounts are silently out of scope, and zero balances
/// are omitted, same convention as the trial balance and balance sheet reports.
pub fn build(ledger: &Ledger) -> IncomeStatementReport {
    let mut by_commodity: BTreeMap<String, Accumulator> = BTreeMap::new();
    let mut unclassified: Vec<Account> = Vec::new();

    for (account, amounts) in ledger.all_leaf_balances() {
        for amount in amounts {
            if amount.quantity.is_zero() {
                continue;
            }
            match classify(&account) {
                Some(AccountType::Revenue) => by_commodity
                    .entry(amount.commodity.clone())
                    .or_default()
                    .revenue
                    .push((account.clone(), -amount.quantity)),
                Some(AccountType::Expense) => by_commodity
                    .entry(amount.commodity.clone())
                    .or_default()
                    .expenses
                    .push((account.clone(), amount.quantity)),
                Some(AccountType::Asset | AccountType::Liability | AccountType::Equity) => {}
                None => unclassified.push(account.clone()),
            }
        }
    }

    unclassified.sort();
    unclassified.dedup();

    let statements = by_commodity
        .into_iter()
        .map(|(commodity, acc)| IncomeStatement {
            commodity,
            revenue: to_section(acc.revenue),
            expenses: to_section(acc.expenses),
        })
        .collect();

    IncomeStatementReport {
        statements,
        unclassified_accounts: unclassified,
    }
}

fn to_section(rows: Vec<(Account, Decimal)>) -> IncomeStatementSection {
    let total = rows.iter().map(|(_, amount)| *amount).sum();
    let rows = rows
        .into_iter()
        .map(|(account, amount)| IncomeStatementRow { account, amount })
        .collect();
    IncomeStatementSection { rows, total }
}

impl IncomeStatementReport {
    /// Renders the report as plain text: one block per commodity (Revenue, Expenses, and the
    /// net income/loss bottom line), followed by a note listing any unclassified accounts.
    pub fn render(&self) -> String {
        let mut out = String::new();
        if self.statements.is_empty() {
            out.push_str("(no postings)\n");
        }
        for statement in &self.statements {
            render_statement(&mut out, statement);
            out.push('\n');
        }
        if !self.unclassified_accounts.is_empty() {
            writeln!(
                out,
                "NOTE: {} account(s) have an unrecognized top-level segment and are excluded from this report:",
                self.unclassified_accounts.len()
            )
            .unwrap();
            for account in &self.unclassified_accounts {
                writeln!(out, "  {}", account).unwrap();
            }
        }
        out
    }
}

fn render_statement(out: &mut String, statement: &IncomeStatement) {
    if !statement.commodity.is_empty() {
        writeln!(out, "Commodity: {}", statement.commodity).unwrap();
    }

    render_section(out, "REVENUE", "Total Revenue", &statement.revenue);
    render_section(out, "EXPENSES", "Total Expenses", &statement.expenses);

    let net_income = statement.net_income();
    let label = if net_income.is_sign_negative() { "NET LOSS" } else { "NET INCOME" };
    writeln!(out, "{}: {:.2}", label, net_income).unwrap();
}

fn render_section(out: &mut String, heading: &str, total_label: &str, section: &IncomeStatementSection) {
    writeln!(out, "{}", heading).unwrap();
    let name_width = section
        .rows
        .iter()
        .map(|r| r.account.0.len())
        .max()
        .unwrap_or(0)
        .max(total_label.len());

    for row in &section.rows {
        writeln!(out, "  {:<nw$}  {:>14.2}", row.account.0, row.amount, nw = name_width).unwrap();
    }
    writeln!(out, "  {:<nw$}  {:>14.2}", total_label, section.total, nw = name_width).unwrap();
    out.push('\n');
}
