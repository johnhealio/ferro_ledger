use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::fmt;

/// A colon-separated account hierarchy, e.g. `Assets:Bank:Checking`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Account(pub String);

impl Account {
    pub fn new(s: impl Into<String>) -> Self {
        Account(s.into())
    }

    pub fn segments(&self) -> Vec<&str> {
        self.0.split(':').collect()
    }

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
/// `Amount::checked_add`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Amount {
    pub quantity: Decimal,
    pub commodity: Commodity,
}

/// Interned-by-value commodity symbol. Empty string means "no symbol" (a bare number).
pub type Commodity = String;

impl Amount {
    pub fn new(quantity: Decimal, commodity: impl Into<Commodity>) -> Self {
        Amount {
            quantity,
            commodity: commodity.into(),
        }
    }

    pub fn zero(commodity: impl Into<Commodity>) -> Self {
        Amount::new(Decimal::ZERO, commodity)
    }

    pub fn is_zero(&self) -> bool {
        self.quantity.is_zero()
    }

    pub fn negate(&self) -> Amount {
        Amount::new(-self.quantity, self.commodity.clone())
    }

    /// Adds two amounts of the same commodity. Returns `None` if commodities differ.
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
    #[default]
    Unmarked,
    Pending,
    Cleared,
}

/// A `key:value` (or bare `key`) tag parsed out of a comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub key: String,
    pub value: Option<String>,
}

impl Tag {
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
    pub file: String,
    pub line: usize,
}

impl fmt::Display for SourcePos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.file, self.line)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Posting {
    pub account: Account,
    /// `None` for an elided posting whose amount the parser inferred to be the balancing
    /// remainder; by the time parsing finishes every posting's `amount` is filled in
    /// (`Transaction::postings` never carries a `None` past `parser::parse`), but the type
    /// stays `Option` because "was this elided in the source" is meaningful for `print`-style
    /// round-tripping later.
    pub amount: Option<Amount>,
    pub was_elided: bool,
    pub status: Option<Status>,
    pub comment: Option<String>,
    pub tags: Vec<Tag>,
    pub source: SourcePos,
}

impl Posting {
    pub fn tag(&self, key: &str) -> Option<&Tag> {
        self.tags.iter().find(|t| t.key == key)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    pub date: NaiveDate,
    pub secondary_date: Option<NaiveDate>,
    pub status: Status,
    pub code: Option<String>,
    pub description: String,
    pub comment: Option<String>,
    pub tags: Vec<Tag>,
    pub postings: Vec<Posting>,
    pub source: SourcePos,
}

impl Transaction {
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
