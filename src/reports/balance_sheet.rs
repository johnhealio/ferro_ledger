//! Balance sheet report: Assets, Liabilities, and Equity, classified from each account's
//! top-level segment (see [`crate::account_types`]). See `docs/BALANCE_SHEET.md` for the full
//! design, including how unclosed Income/Expense activity is folded into Equity as a "Net
//! Income (unclosed)" line so the sheet actually balances.

use crate::account_types::{classify, AccountType};
use crate::ledger::Ledger;
use crate::model::Account;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// The synthetic account label used for the folded-in, not-yet-closed net income/loss line in
/// the Equity section. Not a real account in the journal — see `docs/BALANCE_SHEET.md`.
const NET_INCOME_LABEL: &str = "Net Income (unclosed)";

/// One account's row in a balance sheet section. `amount` is already sign-flipped as needed so
/// a normal balance for that section reads as a positive number (see [`build`]).
pub struct BalanceSheetRow {
    /// The account (or, for the folded-in net income line, a synthetic label —
    /// `"Net Income (unclosed)"`, this module's `NET_INCOME_LABEL`).
    pub account: String,
    /// The display amount: positive for a normal balance in this section.
    pub amount: Decimal,
}

/// One section (Assets, Liabilities, or Equity) of a balance sheet for a single commodity.
pub struct BalanceSheetSection {
    /// Rows in account order (the synthetic net income row, if present, sorts last).
    pub rows: Vec<BalanceSheetRow>,
    /// Sum of every row's `amount`.
    pub total: Decimal,
}

/// A full balance sheet for one commodity: Assets, Liabilities, and Equity, with the
/// accounting-equation check in [`BalanceSheet::is_balanced`].
pub struct BalanceSheet {
    /// The commodity/currency symbol this sheet reports on.
    pub commodity: String,
    /// Accounts classified as [`AccountType::Asset`].
    pub assets: BalanceSheetSection,
    /// Accounts classified as [`AccountType::Liability`], sign-flipped to display positive.
    pub liabilities: BalanceSheetSection,
    /// Accounts classified as [`AccountType::Equity`] (sign-flipped) plus the folded-in net
    /// income row.
    pub equity: BalanceSheetSection,
}

impl BalanceSheet {
    /// True if Assets == Liabilities + Equity for this commodity.
    pub fn is_balanced(&self) -> bool {
        self.assets.total == self.liabilities.total + self.equity.total
    }
}

/// The full report: one [`BalanceSheet`] per commodity, plus any accounts the classifier
/// couldn't place on either side of the equation (see [`crate::account_types::classify`]) —
/// these are omitted from every sheet above, and listed here so they aren't silently dropped.
pub struct BalanceSheetReport {
    /// One sheet per commodity with any nonzero Asset/Liability/Equity/Income/Expense balance.
    pub sheets: Vec<BalanceSheet>,
    /// Accounts with a nonzero balance whose top-level segment didn't match a recognized
    /// account type, sorted and de-duplicated. Not shown on any sheet.
    pub unclassified_accounts: Vec<Account>,
}

impl BalanceSheetReport {
    /// True if every sheet balances.
    pub fn is_balanced(&self) -> bool {
        self.sheets.iter().all(BalanceSheet::is_balanced)
    }
}

#[derive(Default)]
struct Accumulator {
    assets: Vec<(String, Decimal)>,
    liabilities: Vec<(String, Decimal)>,
    equity: Vec<(String, Decimal)>,
    income_total: Decimal,
    expense_total: Decimal,
}

/// Builds a balance sheet from a ledger's leaf-account balances. Accounts are classified by
/// [`classify`]; Income and Expense balances aren't shown directly but are netted into a single
/// "Net Income (unclosed)" row in Equity (see `docs/BALANCE_SHEET.md` for why). Zero balances
/// are omitted, same convention as the trial balance report.
pub fn build(ledger: &Ledger) -> BalanceSheetReport {
    let mut by_commodity: BTreeMap<String, Accumulator> = BTreeMap::new();
    let mut unclassified: Vec<Account> = Vec::new();

    for (account, amounts) in ledger.all_leaf_balances() {
        for amount in amounts {
            if amount.quantity.is_zero() {
                continue;
            }
            let acc = by_commodity.entry(amount.commodity.clone()).or_default();
            match classify(&account) {
                Some(AccountType::Asset) => acc.assets.push((account.0.clone(), amount.quantity)),
                Some(AccountType::Liability) => acc.liabilities.push((account.0.clone(), -amount.quantity)),
                Some(AccountType::Equity) => acc.equity.push((account.0.clone(), -amount.quantity)),
                Some(AccountType::Revenue) => acc.income_total += amount.quantity,
                Some(AccountType::Expense) => acc.expense_total += amount.quantity,
                None => unclassified.push(account.clone()),
            }
        }
    }

    unclassified.sort();
    unclassified.dedup();

    let sheets = by_commodity
        .into_iter()
        .map(|(commodity, acc)| build_sheet(commodity, acc))
        .collect();

    BalanceSheetReport {
        sheets,
        unclassified_accounts: unclassified,
    }
}

fn build_sheet(commodity: String, acc: Accumulator) -> BalanceSheet {
    let assets = to_section(acc.assets);
    let liabilities = to_section(acc.liabilities);

    let mut equity_rows = acc.equity;
    let net_income = -(acc.income_total + acc.expense_total);
    if !net_income.is_zero() {
        equity_rows.push((NET_INCOME_LABEL.to_string(), net_income));
    }
    let equity = to_section(equity_rows);

    BalanceSheet {
        commodity,
        assets,
        liabilities,
        equity,
    }
}

fn to_section(rows: Vec<(String, Decimal)>) -> BalanceSheetSection {
    let total = rows.iter().map(|(_, amount)| *amount).sum();
    let rows = rows
        .into_iter()
        .map(|(account, amount)| BalanceSheetRow { account, amount })
        .collect();
    BalanceSheetSection { rows, total }
}

impl BalanceSheetReport {
    /// Renders the report as plain text: one block per commodity (Assets, Liabilities, Equity,
    /// and the accounting-equation check), followed by a warning listing any unclassified
    /// accounts that were left out.
    pub fn render(&self) -> String {
        let mut out = String::new();
        if self.sheets.is_empty() {
            out.push_str("(no postings)\n");
        }
        for sheet in &self.sheets {
            render_sheet(&mut out, sheet);
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

fn render_sheet(out: &mut String, sheet: &BalanceSheet) {
    if !sheet.commodity.is_empty() {
        writeln!(out, "Commodity: {}", sheet.commodity).unwrap();
    }

    render_section(out, "ASSETS", "Total Assets", &sheet.assets);
    render_section(out, "LIABILITIES", "Total Liabilities", &sheet.liabilities);
    render_section(out, "EQUITY", "Total Equity", &sheet.equity);

    writeln!(
        out,
        "Assets ({:.2}) {} Liabilities + Equity ({:.2})",
        sheet.assets.total,
        if sheet.is_balanced() { "=" } else { "!=" },
        sheet.liabilities.total + sheet.equity.total
    )
    .unwrap();
    if !sheet.is_balanced() {
        out.push_str("WARNING: balance sheet does not balance.\n");
    }
}

fn render_section(out: &mut String, heading: &str, total_label: &str, section: &BalanceSheetSection) {
    writeln!(out, "{}", heading).unwrap();
    let name_width = section
        .rows
        .iter()
        .map(|r| r.account.len())
        .max()
        .unwrap_or(0)
        .max(total_label.len());

    for row in &section.rows {
        writeln!(out, "  {:<nw$}  {:>14.2}", row.account, row.amount, nw = name_width).unwrap();
    }
    writeln!(out, "  {:<nw$}  {:>14.2}", total_label, section.total, nw = name_width).unwrap();
    out.push('\n');
}
