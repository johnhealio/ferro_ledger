//! `ferro_ledger` binary entry point: parses CLI args (see [`ferro_ledger::cli`]) and dispatches
//! to one of the `run_*` functions below. Deliberately thin — argument definitions live in
//! `cli.rs` and all report logic lives in the library crate, so nothing here needs its own
//! tests.

use clap::Parser;
use ferro_ledger::cli::{Cli, Command};
use ferro_ledger::ledger::Ledger;
use ferro_ledger::model::Account;
use ferro_ledger::reports::{balance_sheet, clearing, trial_balance};
use ferro_ledger::journal::load_journal;

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Balance { file } => run_balance(&file),
        Command::BalanceSheet { file } => run_balance_sheet(&file),
        Command::Check { file } => run_check(&file),
        Command::Clear { file, accounts } => run_clear(&file, &accounts),
    };

    if let Err(message) = result {
        eprintln!("error: {}", message);
        std::process::exit(1);
    }
}

/// Runs the `balance`/`trial-balance` subcommand: loads `file`, prints its trial balance, and
/// fails (nonzero exit) if any commodity section doesn't foot.
fn run_balance(file: &std::path::Path) -> Result<(), String> {
    let transactions = load_journal(file).map_err(|e| e.to_string())?;
    let ledger = Ledger::from_transactions(transactions);
    let report = trial_balance::build(&ledger);
    print!("{}", report.render());
    if !report.is_balanced() {
        return Err("trial balance does not foot".to_string());
    }
    Ok(())
}

/// Runs the `balance-sheet`/`bs` subcommand: loads `file`, prints its balance sheet, and fails
/// (nonzero exit) if any commodity's sheet doesn't balance.
fn run_balance_sheet(file: &std::path::Path) -> Result<(), String> {
    let transactions = load_journal(file).map_err(|e| e.to_string())?;
    let ledger = Ledger::from_transactions(transactions);
    let report = balance_sheet::build(&ledger);
    print!("{}", report.render());
    if !report.is_balanced() {
        return Err("balance sheet does not balance".to_string());
    }
    Ok(())
}

/// Runs the `check` subcommand: loads and balance-validates `file`, printing nothing on success.
fn run_check(file: &std::path::Path) -> Result<(), String> {
    load_journal(file).map_err(|e| e.to_string())?;
    Ok(())
}

/// Runs the `clear` subcommand: loads `file` and prints a clearing-group analysis for each
/// named account (see [`ferro_ledger::reports::clearing`]).
fn run_clear(file: &std::path::Path, accounts: &[String]) -> Result<(), String> {
    let transactions = load_journal(file).map_err(|e| e.to_string())?;
    let ledger = Ledger::from_transactions(transactions);
    let accounts: Vec<Account> = accounts.iter().map(Account::new).collect();
    let reports = clearing::analyze(&ledger, &accounts);
    print!("{}", clearing::render(&reports));
    Ok(())
}
