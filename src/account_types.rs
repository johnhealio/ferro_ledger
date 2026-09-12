//! Heuristic classification of an account's top-level type (Asset/Liability/Equity/Revenue/
//! Expense), used by reports — currently just the balance sheet — that need to group accounts
//! by which side of the accounting equation they belong to.
//!
//! ferro_ledger has no `account` "type" declarations in v1 (see `docs/JOURNAL_FORMAT.md`'s
//! "Directives" section: `account` is parsed and accepted, but carries no type information) —
//! this is a name-based heuristic instead, matching hledger's own default account-type
//! inference: look at the account's top-level segment, case-insensitively, singular or plural.
//! See `docs/BALANCE_SHEET.md` for how the balance sheet report uses this.

use crate::model::Account;

/// Which side of the accounting equation an account belongs to, inferred from its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountType {
    /// Top-level segment `asset`/`assets` (case-insensitive).
    Asset,
    /// Top-level segment `liability`/`liabilities`.
    Liability,
    /// Top-level segment `equity`.
    Equity,
    /// Top-level segment `income`/`revenue`/`revenues`.
    Revenue,
    /// Top-level segment `expense`/`expenses`.
    Expense,
}

/// Classifies `account` by its top-level segment. Returns `None` for anything that doesn't
/// match one of the recognized names — such an account is invisible to the balance sheet report
/// (see `docs/BALANCE_SHEET.md`, "Accounts the classifier doesn't recognize").
pub fn classify(account: &Account) -> Option<AccountType> {
    let top = account.segments().first().copied().unwrap_or("").to_ascii_lowercase();
    match top.as_str() {
        "asset" | "assets" => Some(AccountType::Asset),
        "liability" | "liabilities" => Some(AccountType::Liability),
        "equity" => Some(AccountType::Equity),
        "income" | "revenue" | "revenues" => Some(AccountType::Revenue),
        "expense" | "expenses" => Some(AccountType::Expense),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_recognized_top_level_segments_case_insensitively() {
        assert_eq!(classify(&Account::new("Assets:Bank:Checking")), Some(AccountType::Asset));
        assert_eq!(classify(&Account::new("LIABILITIES:Loans")), Some(AccountType::Liability));
        assert_eq!(classify(&Account::new("equity:Opening")), Some(AccountType::Equity));
        assert_eq!(classify(&Account::new("Revenue:Sales")), Some(AccountType::Revenue));
        assert_eq!(classify(&Account::new("expenses:Rent")), Some(AccountType::Expense));
    }

    #[test]
    fn unrecognized_top_level_segment_is_unclassified() {
        assert_eq!(classify(&Account::new("Misc:Whatever")), None);
    }
}
