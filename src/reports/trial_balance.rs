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

pub struct TrialBalanceRow {
    pub account: Account,
    pub debit: Decimal,
    pub credit: Decimal,
}

pub struct TrialBalanceSection {
    pub commodity: String,
    pub rows: Vec<TrialBalanceRow>,
    pub total_debit: Decimal,
    pub total_credit: Decimal,
}

impl TrialBalanceSection {
    pub fn is_balanced(&self) -> bool {
        self.total_debit == self.total_credit
    }
}

pub struct TrialBalance {
    pub sections: Vec<TrialBalanceSection>,
}

impl TrialBalance {
    pub fn is_balanced(&self) -> bool {
        self.sections.iter().all(TrialBalanceSection::is_balanced)
    }
}

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
