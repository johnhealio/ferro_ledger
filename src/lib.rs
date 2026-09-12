//! **ferro_ledger**: a text-based, command-line general ledger.
//!
//! Journal files are a compatible subset of [hledger](https://hledger.org)'s plain-text format
//! (see `docs/JOURNAL_FORMAT.md`) — there is no database, ever; the journal text file on disk is
//! the single source of truth, re-read and re-computed on every run (see `CLAUDE.md`).
//!
//! # Crate layout
//!
//! - [`model`] — core journal types: [`model::Account`], [`model::Amount`], [`model::Posting`],
//!   [`model::Transaction`].
//! - [`parser`] — hand-written journal text -> [`model::Transaction`] parser. Pure, no I/O.
//! - [`journal`] — loads a journal file from disk, resolving `include` directives; the only
//!   place file I/O happens.
//! - [`ledger`] — [`ledger::Ledger`], an in-memory index over parsed transactions with
//!   account-balance queries.
//! - [`reports`] — [`reports::trial_balance`] (the primary report), [`reports::balance_sheet`]
//!   (see `docs/BALANCE_SHEET.md`), and [`reports::clearing`] (clearing/suspense-account
//!   matching — see `docs/CLEARING_ACCOUNTS.md`).
//! - [`account_types`] — the name-based heuristic the balance sheet uses to classify accounts
//!   as Asset/Liability/Equity/Revenue/Expense.
//! - [`cli`] — the `clap`-derived command-line argument definitions used by the `ferro_ledger`
//!   binary (`src/main.rs`).
//!
//! See `docs/ARCHITECTURE.md` for how these pieces fit together, and `docs/PLANNING.md` for
//! project status.

pub mod account_types;
pub mod cli;
pub mod journal;
pub mod ledger;
pub mod model;
pub mod parser;
pub mod reports;
