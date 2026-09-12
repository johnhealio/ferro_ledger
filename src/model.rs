//! Core journal data types: [`Account`], [`Amount`], [`Posting`], [`Transaction`], plus the
//! small supporting types ([`Status`], [`Tag`], [`SourcePos`]) they're built from.
//!
//! Everything here is plain, immutable-by-convention data — no I/O, no parsing logic. See
//! [`crate::parser`] for how journal text becomes these types, and `docs/JOURNAL_FORMAT.md` for
//! the journal syntax they represent.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::fmt;

/// A colon-separated account hierarchy, e.g. `Assets:Bank:Checking`.
///
/// Accounts are plain strings under the hood (hledger doesn't require accounts to be declared
/// before use, and neither does ferro_ledger in v1) — this type exists so hierarchy logic
/// (parent/descendant checks, splitting into segments) lives in one place instead of being
/// re-derived on raw `String`s throughout the codebase.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Account(pub String);

impl Account {
    /// Wraps any string-like value as an `Account`. Does not validate or normalize it.
    pub fn new(s: impl Into<String>) -> Self {
        Account(s.into())
    }

    /// Splits the account name on `:` into its hierarchy segments, e.g.
    /// `Assets:Bank:Checking` -> `["Assets", "Bank", "Checking"]`.
    pub fn segments(&self) -> Vec<&str> {
        self.0.split(':').collect()
    }

    /// Number of `:`-separated segments in the account name (at least 1).
    pub fn depth(&self) -> usize {
        self.segments().len()
    }

    /// True if `self` is `other` or a descendant of `other` (e.g. `Assets:Bank:Checking`
    /// is under `Assets:Bank` and under `Assets`).
    pub fn is_under(&self, other: &Account) -> bool {
        self.0 == other.0 || self.0.starts_with(&format!("{}:", other.0))
    }

    /// All ancestor accounts from root to self, inclusive, e.g. for `Assets:Bank:Checking`:
    /// `[Assets, Assets:Bank, Assets:Bank:Checking]`.
    pub fn ancestors_inclusive(&self) -> Vec<Account> {
        let segments = self.segments();
        (1..=segments.len())
            .map(|n| Account(segments[..n].join(":")))
            .collect()
    }
}

impl fmt::Display for Account {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A quantity in a single commodity/currency. No cross-commodity arithmetic in v1: adding two
/// `Amount`s of different commodities is a logic error the caller must avoid (see
/// [`Amount::checked_add`], which returns `None` instead of panicking or silently mixing units).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Amount {
    /// The numeric quantity. Always `rust_decimal::Decimal`, never a float — see `CLAUDE.md`.
    pub quantity: Decimal,
    /// The commodity/currency symbol (`"USD"`, `"$"`, or `""` for a bare, symbol-less number).
    pub commodity: Commodity,
}

/// A commodity/currency symbol, stored as its own string per `Amount` (no global commodity
/// table in v1 — see "Deliberately unsupported in v1" in `docs/JOURNAL_FORMAT.md`).
pub type Commodity = String;

impl Amount {
    /// Builds an amount from a quantity and a commodity symbol.
    pub fn new(quantity: Decimal, commodity: impl Into<Commodity>) -> Self {
        Amount {
            quantity,
            commodity: commodity.into(),
        }
    }

    /// A zero quantity in the given commodity.
    pub fn zero(commodity: impl Into<Commodity>) -> Self {
        Amount::new(Decimal::ZERO, commodity)
    }

    /// True if the quantity is exactly zero (regardless of commodity).
    pub fn is_zero(&self) -> bool {
        self.quantity.is_zero()
    }

    /// Returns the same amount with its quantity's sign flipped.
    pub fn negate(&self) -> Amount {
        Amount::new(-self.quantity, self.commodity.clone())
    }

    /// Adds two amounts of the same commodity. Returns `None` if commodities differ, rather than
    /// producing a nonsensical mixed-unit sum.
    pub fn checked_add(&self, other: &Amount) -> Option<Amount> {
        if self.commodity != other.commodity {
            return None;
        }
        Some(Amount::new(self.quantity + other.quantity, self.commodity.clone()))
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.commodity.is_empty() {
            write!(f, "{}", self.quantity)
        } else {
            write!(f, "{} {}", self.quantity, self.commodity)
        }
    }
}

/// Transaction-level (and optionally posting-level) status, hledger's `*`/`!`/unmarked.
///
/// This is distinct from clearing-account group matching — see
/// `docs/CLEARING_ACCOUNTS.md`, "Relationship to hledger's transaction `*`/`!` status".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Status {
    /// No status marker in the source (the common case).
    #[default]
    Unmarked,
    /// Marked `!` in the source — provisionally confirmed, not yet fully reconciled.
    Pending,
    /// Marked `*` in the source — confirmed/reconciled.
    Cleared,
}

/// A `key:value` (or bare `key:` with an empty value) tag parsed out of a comment. See "Comments
/// and tags" in `docs/JOURNAL_FORMAT.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    /// The tag name, e.g. `match` in `match:INV-2044`.
    pub key: String,
    /// The tag's value, or `None` for a bare `key:` with nothing after the colon.
    pub value: Option<String>,
}

impl Tag {
    /// Builds a tag from a key and optional value.
    pub fn new(key: impl Into<String>, value: Option<String>) -> Self {
        Tag {
            key: key.into(),
            value,
        }
    }
}

/// Where a piece of journal data came from, for error messages and report traceability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePos {
    /// The journal file this data was read from, as passed to the parser (may be relative).
    pub file: String,
    /// 1-based line number within `file`.
    pub line: usize,
}

impl fmt::Display for SourcePos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.file, self.line)
    }
}

/// One line of a transaction: an account and the amount moved into/out of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Posting {
    /// The account this posting affects.
    pub account: Account,
    /// `None` for an elided posting whose amount the parser inferred to be the balancing
    /// remainder; by the time parsing finishes every posting's `amount` is filled in
    /// (`Transaction::postings` never carries a `None` past `parser::parse`), but the type
    /// stays `Option` because "was this elided in the source" is meaningful for `print`-style
    /// round-tripping later.
    pub amount: Option<Amount>,
    /// Whether this posting's amount was elided in the source and inferred by the parser (see
    /// `amount` above — inferred amounts are still filled into `amount`, this flag is what
    /// distinguishes "written explicitly" from "inferred").
    pub was_elided: bool,
    /// An optional per-posting status marker (hledger allows `*`/`!` on individual postings, not
    /// just the whole transaction).
    pub status: Option<Status>,
    /// The raw comment text following `;` on this posting's line, if any.
    pub comment: Option<String>,
    /// Tags parsed out of `comment`.
    pub tags: Vec<Tag>,
    /// File/line this posting was parsed from.
    pub source: SourcePos,
}

impl Posting {
    /// Looks up a tag on this posting by key (not falling back to the parent transaction's tags
    /// — see [`Transaction::effective_tag`] for that).
    pub fn tag(&self, key: &str) -> Option<&Tag> {
        self.tags.iter().find(|t| t.key == key)
    }
}

/// A balanced group of postings recorded together, hledger's fundamental journal entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    /// The transaction's primary date.
    pub date: NaiveDate,
    /// An optional secondary date after `=` in the source (e.g. a settlement date distinct from
    /// the recorded date).
    pub secondary_date: Option<NaiveDate>,
    /// The transaction-level status marker (`*`/`!`/unmarked).
    pub status: Status,
    /// An optional parenthesized reference code, e.g. `(CHK-101)`.
    pub code: Option<String>,
    /// The free-text description following the date/status/code.
    pub description: String,
    /// The raw comment text on the transaction's own header/comment lines, if any.
    pub comment: Option<String>,
    /// Tags parsed out of `comment`.
    pub tags: Vec<Tag>,
    /// This transaction's postings. Always sums to zero per commodity — enforced at parse time.
    pub postings: Vec<Posting>,
    /// File/line the transaction header was parsed from.
    pub source: SourcePos,
}

impl Transaction {
    /// Looks up a tag on this transaction by key.
    pub fn tag(&self, key: &str) -> Option<&Tag> {
        self.tags.iter().find(|t| t.key == key)
    }

    /// Effective tags for a posting: the posting's own tags, falling back to the parent
    /// transaction's tags for any key the posting doesn't set itself. Used by clearing-group
    /// matching, which looks for a `match:` tag wherever it was placed.
    pub fn effective_tag<'a>(&'a self, posting: &'a Posting, key: &str) -> Option<&'a Tag> {
        posting.tag(key).or_else(|| self.tag(key))
    }
}
