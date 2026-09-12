use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "ferro_ledger",
    version,
    about = "A text-based, hledger-journal-compatible general ledger"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Print a trial balance: one row per account with debit/credit columns.
    #[command(alias = "trial-balance")]
    Balance {
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
