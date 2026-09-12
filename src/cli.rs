//! Command-line argument definitions ([`clap`] derive), kept separate from `main.rs` so the
//! argument surface is easy to scan in one place and unit-testable independent of process I/O.

use clap::{Parser, Subcommand};
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

/// The available subcommands, one per report/action `main.rs` can run.
#[derive(Subcommand)]
pub enum Command {
    /// Print a trial balance: one row per account with debit/credit columns.
    #[command(alias = "trial-balance")]
    Balance {
        /// Path to the journal file.
        file: PathBuf,
    },

    /// Print a balance sheet: Assets, Liabilities, and Equity, classified from each account's
    /// top-level segment. See docs/BALANCE_SHEET.md.
    #[command(alias = "bs")]
    BalanceSheet {
        /// Path to the journal file.
        file: PathBuf,
    },

    /// Parse and balance-check the journal only; prints nothing and exits 0 on success, or
    /// prints the parse error and exits non-zero.
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
    },
}
