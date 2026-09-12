//! Command-line argument definitions ([`clap`] derive), kept separate from `main.rs` so the
//! argument surface is easy to scan in one place and unit-testable independent of process I/O.

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

/// Top-level CLI: `ferro_ledger <SUBCOMMAND> ...`. Parsed with [`clap::Parser::parse`] in
/// `main.rs`; this type is otherwise inert data, not part of the reporting logic.
#[derive(Parser)]
#[command(
    name = "ferro_ledger",
    version,
    about = "A text-based, hledger-journal-compatible general ledger"
)]
pub struct Cli {
    /// Which report/action to run.
    #[command(subcommand)]
    pub command: Command,
}

/// The `--since`/`--until`/`--period` date-range flags shared by every report subcommand that
/// builds a [`crate::ledger::Ledger`] (see `docs/DATE_RANGE.md`). Flattened into each such
/// variant of [`Command`] rather than duplicated, so the flags always mean the same thing
/// everywhere.
#[derive(Args, Debug, Clone, Default)]
pub struct DateRangeArgs {
    /// Only include transactions on or after this date (inclusive). Accepts YYYY-MM-DD,
    /// YYYY/MM/DD, or YYYY.MM.DD — the same formats journal dates use. Cannot be combined with
    /// `--period`.
    #[arg(long, conflicts_with = "period")]
    pub since: Option<String>,

    /// Only include transactions strictly before this date (exclusive). Same accepted formats
    /// as `--since`. Cannot be combined with `--period`.
    #[arg(long, conflicts_with = "period")]
    pub until: Option<String>,

    /// Shorthand for a whole period, expanding to an equivalent `--since`/`--until` pair:
    /// `YYYY` (a calendar year), `YYYY-MM` (a calendar month), `YYYY-MM-DD` (a single day), or
    /// `["from"] TERM "to" TERM` for a range of those (e.g. "2024-01 to 2024-03"). See
    /// docs/DATE_RANGE.md. Cannot be combined with `--since`/`--until`.
    #[arg(long)]
    pub period: Option<String>,
}

/// The available subcommands, one per report/action `main.rs` can run.
#[derive(Subcommand)]
pub enum Command {
    /// Print a trial balance: one row per account with debit/credit columns.
    #[command(alias = "trial-balance")]
    Balance {
        /// Path to the journal file.
        file: PathBuf,

        /// `--since`/`--until`/`--period` date-range scoping.
        #[command(flatten)]
        date_range: DateRangeArgs,
    },

    /// Print a balance sheet: Assets, Liabilities, and Equity, classified from each account's
    /// top-level segment. See docs/BALANCE_SHEET.md.
    #[command(alias = "bs")]
    BalanceSheet {
        /// Path to the journal file.
        file: PathBuf,

        /// `--since`/`--until`/`--period` date-range scoping.
        #[command(flatten)]
        date_range: DateRangeArgs,
    },

    /// Print an income statement (profit & loss): Revenue and Expenses, ending in a net
    /// income/loss line. See docs/INCOME_STATEMENT.md.
    #[command(alias = "is")]
    IncomeStatement {
        /// Path to the journal file.
        file: PathBuf,

        /// `--since`/`--until`/`--period` date-range scoping.
        #[command(flatten)]
        date_range: DateRangeArgs,
    },

    /// Parse and balance-check the journal only; prints nothing and exits 0 on success, or
    /// prints the parse error and exits non-zero. Not scoped by `--since`/`--until` — every
    /// transaction is balance-checked regardless of date.
    Check {
        /// Path to the journal file.
        file: PathBuf,
    },

    /// Analyze clearing/suspense account(s): group postings by matching key and report cleared
    /// vs. outstanding groups. See docs/CLEARING_ACCOUNTS.md.
    Clear {
        /// Path to the journal file.
        file: PathBuf,

        /// Clearing account to analyze. Repeat for multiple accounts.
        #[arg(long = "account", required = true)]
        accounts: Vec<String>,

        /// `--since`/`--until`/`--period` date-range scoping.
        #[command(flatten)]
        date_range: DateRangeArgs,
    },
}
