//! The primary report: a classic trial balance — one row per account (in debit or credit
//! column depending on sign), with a total row that must foot to the same value on both sides.
//!
//! Amounts are never summed across commodities (see `docs/JOURNAL_FORMAT.md`): a journal using
//! more than one commodity gets one section per commodity, each independently balanced.

use crate::ledger::Ledger;
use crate::model::Account;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// One account's row in a trial balance: its net balance, split into a debit or a credit column
/// by sign. Exactly one of `debit`/`credit` is nonzero (a row is only created when the
/// account's net balance is nonzero — see [`build`]).
pub struct TrialBalanceRow {
    /// The account this row reports on.
    pub account: Account,
    /// Nonzero if this account's net balance is positive (a debit balance).
    pub debit: Decimal,
    /// Nonzero if this account's net balance is negative (a credit balance).
    pub credit: Decimal,
}

/// A trial balance for a single commodity: every account with a nonzero balance in that
/// commodity, plus totals. See the module docs for why commodities are never mixed into one
/// section.
pub struct TrialBalanceSection {
    /// The commodity/currency symbol this section reports on.
    pub commodity: String,
    /// One row per account with a nonzero balance, in account order.
    pub rows: Vec<TrialBalanceRow>,
    /// Sum of every row's `debit` column.
    pub total_debit: Decimal,
    /// Sum of every row's `credit` column.
    pub total_credit: Decimal,
}

impl TrialBalanceSection {
    /// True if this section's totals foot (debits equal credits). Should always be true if the
    /// underlying journal balanced at parse time — this is an independent cross-check, not the
    /// primary place balancing is enforced (see `parser::balance_transaction`).
    pub fn is_balanced(&self) -> bool {
        self.total_debit == self.total_credit
    }
}

/// The full trial balance report: one [`TrialBalanceSection`] per commodity found in the ledger.
pub struct TrialBalance {
    /// Sections in commodity order, one per distinct commodity with any nonzero balance.
    pub sections: Vec<TrialBalanceSection>,
}

impl TrialBalance {
    /// True if every section foots. See [`TrialBalanceSection::is_balanced`].
    pub fn is_balanced(&self) -> bool {
        self.sections.iter().all(TrialBalanceSection::is_balanced)
    }
}

/// Builds a trial balance from a ledger's leaf-account balances: one row per account with a
/// nonzero net balance, grouped into one section per commodity. Accounts that net exactly to
/// zero are omitted, per standard trial-balance convention.
pub fn build(ledger: &Ledger) -> TrialBalance {
    let mut by_commodity: BTreeMap<String, Vec<TrialBalanceRow>> = BTreeMap::new();

    for (account, amounts) in ledger.all_leaf_balances() {
        for amount in amounts {
            if amount.quantity.is_zero() {
                continue;
            }
            let (debit, credit) = if amount.quantity.is_sign_positive() {
                (amount.quantity, Decimal::ZERO)
            } else {
                (Decimal::ZERO, -amount.quantity)
            };
            by_commodity
                .entry(amount.commodity.clone())
                .or_default()
                .push(TrialBalanceRow {
                    account: account.clone(),
                    debit,
                    credit,
                });
        }
    }

    let sections = by_commodity
        .into_iter()
        .map(|(commodity, rows)| {
            let total_debit = rows.iter().map(|r| r.debit).sum();
            let total_credit = rows.iter().map(|r| r.credit).sum();
            TrialBalanceSection {
                commodity,
                rows,
                total_debit,
                total_credit,
            }
        })
        .collect();

    TrialBalance { sections }
}

impl TrialBalance {
    /// Renders the report as an aligned plain-text table (one block per commodity section),
    /// suitable for printing directly to a terminal.
    pub fn render(&self) -> String {
        let mut out = String::new();
        if self.sections.is_empty() {
            out.push_str("(no postings)\n");
            return out;
        }
        for section in &self.sections {
            render_section(&mut out, section);
            out.push('\n');
        }
        out
    }
}

fn render_section(out: &mut String, section: &TrialBalanceSection) {
    if !section.commodity.is_empty() {
        writeln!(out, "Commodity: {}", section.commodity).unwrap();
    }

    let account_width = section
        .rows
        .iter()
        .map(|r| r.account.0.len())
        .max()
        .unwrap_or(0)
        .max("Account".len());

    writeln!(
        out,
        "{:<aw$}  {:>14}  {:>14}",
        "Account",
        "Debit",
        "Credit",
        aw = account_width
    )
    .unwrap();
    writeln!(out, "{}", "-".repeat(account_width + 2 + 14 + 2 + 14)).unwrap();

    for row in &section.rows {
        writeln!(
            out,
            "{:<aw$}  {:>14}  {:>14}",
            row.account.0,
            fmt_or_blank(row.debit),
            fmt_or_blank(row.credit),
            aw = account_width
        )
        .unwrap();
    }

    writeln!(out, "{}", "-".repeat(account_width + 2 + 14 + 2 + 14)).unwrap();
    writeln!(
        out,
        "{:<aw$}  {:>14}  {:>14}",
        "TOTAL",
        format!("{:.2}", section.total_debit),
        format!("{:.2}", section.total_credit),
        aw = account_width
    )
    .unwrap();

    if !section.is_balanced() {
        writeln!(
            out,
            "WARNING: trial balance does not foot — debits {} != credits {}",
            section.total_debit, section.total_credit
        )
        .unwrap();
    }
}

fn fmt_or_blank(value: Decimal) -> String {
    if value.is_zero() {
        String::new()
    } else {
        format!("{:.2}", value)
    }
}
