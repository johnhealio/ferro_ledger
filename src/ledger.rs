//! In-memory ledger built fresh from parsed transactions on every run — no cached/persisted
//! state beyond the lifetime of one CLI invocation (see `CLAUDE.md`: no database, ever).

use crate::model::{Account, Amount, Posting, Transaction};
use std::collections::BTreeMap;

/// A journal's transactions, held in memory for the lifetime of one report. Build with
/// [`Ledger::from_transactions`]; everything else is a read-only query over that data.
pub struct Ledger {
    /// All transactions, sorted by date (then by source line, for a stable order within a day).
    pub transactions: Vec<Transaction>,
}

/// A posting together with the transaction it belongs to, as yielded by [`Ledger::postings`].
/// Carrying the parent transaction alongside the posting is what lets reports show a posting's
/// date/description/tags without a separate lookup.
#[derive(Clone, Copy)]
pub struct PostingRef<'a> {
    /// The transaction this posting belongs to.
    pub transaction: &'a Transaction,
    /// The posting itself.
    pub posting: &'a Posting,
}

impl Ledger {
    /// Builds a `Ledger` from parsed transactions, sorting them by date (ties broken by source
    /// line) for stable, chronological report output.
    pub fn from_transactions(mut transactions: Vec<Transaction>) -> Self {
        transactions.sort_by(|a, b| a.date.cmp(&b.date).then_with(|| a.source.line.cmp(&b.source.line)));
        Ledger { transactions }
    }

    /// Iterates every posting in every transaction, each paired with its parent transaction.
    pub fn postings(&self) -> impl Iterator<Item = PostingRef<'_>> {
        self.transactions
            .iter()
            .flat_map(|t| t.postings.iter().map(move |p| PostingRef { transaction: t, posting: p }))
    }

    /// Postings whose account is `account` or a descendant of it (colon-hierarchy rollup).
    pub fn postings_in<'a>(&'a self, account: &'a Account) -> impl Iterator<Item = PostingRef<'a>> {
        self.postings().filter(move |pr| pr.posting.account.is_under(account))
    }

    /// Every distinct account that appears literally on a posting, sorted.
    pub fn accounts(&self) -> Vec<Account> {
        let mut set: Vec<Account> = self.postings().map(|pr| pr.posting.account.clone()).collect();
        set.sort();
        set.dedup();
        set
    }

    /// Sums, per commodity, of every posting made directly to exactly this account (no rollup
    /// into descendants) — what a trial balance row shows.
    pub fn leaf_balance(&self, account: &Account) -> Vec<Amount> {
        let mut totals: Vec<Amount> = Vec::new();
        for pr in self.postings() {
            if pr.posting.account == *account
                && let Some(amount) = &pr.posting.amount {
                    add_into(&mut totals, amount);
                }
        }
        totals
    }

    /// Sums, per commodity, of every posting made to this account or any descendant account —
    /// the hierarchical subtotal hledger shows for a parent like `Assets`.
    pub fn subtree_balance(&self, account: &Account) -> Vec<Amount> {
        let mut totals: Vec<Amount> = Vec::new();
        for pr in self.postings_in(account) {
            if let Some(amount) = &pr.posting.amount {
                add_into(&mut totals, amount);
            }
        }
        totals
    }

    /// Leaf balances for every account that has at least one posting, in account order.
    pub fn all_leaf_balances(&self) -> BTreeMap<Account, Vec<Amount>> {
        let mut map: BTreeMap<Account, Vec<Amount>> = BTreeMap::new();
        for pr in self.postings() {
            if let Some(amount) = &pr.posting.amount {
                add_into(map.entry(pr.posting.account.clone()).or_default(), amount);
            }
        }
        map
    }
}

fn add_into(totals: &mut Vec<Amount>, amount: &Amount) {
    match totals.iter_mut().find(|a| a.commodity == amount.commodity) {
        Some(existing) => existing.quantity += amount.quantity,
        None => totals.push(amount.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_str;

    fn ledger_from(src: &str) -> Ledger {
        let parsed = parse_str(src, "test.journal").expect("should parse");
        Ledger::from_transactions(parsed.transactions)
    }

    #[test]
    fn subtree_balance_rolls_up_descendants() {
        let ledger = ledger_from(
            "2024-01-01 Opening\n    Assets:Bank:Checking   100.00 USD\n    Assets:Bank:Savings     50.00 USD\n    Equity:OpeningBalance -150.00 USD\n",
        );
        let total = ledger.subtree_balance(&Account::new("Assets"));
        assert_eq!(total.len(), 1);
        assert_eq!(total[0].quantity, rust_decimal::Decimal::new(15000, 2));
    }

    #[test]
    fn leaf_balance_does_not_roll_up() {
        let ledger = ledger_from(
            "2024-01-01 Opening\n    Assets:Bank:Checking   100.00 USD\n    Equity:OpeningBalance -100.00 USD\n",
        );
        let assets_total = ledger.leaf_balance(&Account::new("Assets"));
        assert!(assets_total.is_empty());
    }
}
