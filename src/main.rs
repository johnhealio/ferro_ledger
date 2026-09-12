//! `ferro_ledger` binary entry point: parses CLI args (see [`ferro_ledger::cli`]) and dispatches
//! to one of the `run_*` functions below. Deliberately thin — argument definitions live in
//! `cli.rs` and all report logic lives in the library crate, so nothing here needs its own
//! tests.

use clap::Parser;
use ferro_ledger::cli::{Cli, Command, DateRangeArgs};
use ferro_ledger::date_range::DateRange;
use ferro_ledger::ledger::Ledger;
use ferro_ledger::model::{Account, Transaction};
use ferro_ledger::parser::parse_date_str;
use ferro_ledger::reports::{balance_sheet, clearing, income_statement, trial_balance};
use ferro_ledger::journal::load_journal;

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Balance { file, date_range } => run_balance(&file, &date_range),
        Command::BalanceSheet { file, date_range } => run_balance_sheet(&file, &date_range),
        Command::IncomeStatement { file, date_range } => run_income_statement(&file, &date_range),
        Command::Check { file } => run_check(&file),
        Command::Clear { file, accounts, date_range } => run_clear(&file, &accounts, &date_range),
    };

    if let Err(message) = result {
        eprintln!("error: {}", message);
        std::process::exit(1);
    }
}

/// Parses `args.since`/`args.until` (if present) into a [`DateRange`], with a clear error
/// naming the flag and the accepted formats if either fails to parse.
fn resolve_date_range(args: &DateRangeArgs) -> Result<DateRange, String> {
    let since = parse_date_flag("--since", &args.since)?;
    let until = parse_date_flag("--until", &args.until)?;
    Ok(DateRange::new(since, until))
}

fn parse_date_flag(flag: &str, value: &Option<String>) -> Result<Option<chrono::NaiveDate>, String> {
    match value {
        None => Ok(None),
        Some(s) => parse_date_str(s)
            .map(Some)
            .ok_or_else(|| format!("invalid {} date '{}': expected YYYY-MM-DD, YYYY/MM/DD, or YYYY.MM.DD", flag, s)),
    }
}

/// Loads `file` and applies `date_range`, printing a one-line period header first if the range
/// is bounded (so report output makes clear it's scoped, not the whole journal).
fn load_and_filter(file: &std::path::Path, date_range: &DateRangeArgs) -> Result<Vec<Transaction>, String> {
    let range = resolve_date_range(date_range)?;
    let transactions = load_journal(file).map_err(|e| e.to_string())?;
    if !range.is_unbounded() {
        println!("{}", render_period_header(&range));
    }
    Ok(range.filter(transactions))
}

fn render_period_header(range: &DateRange) -> String {
    match (range.since, range.until) {
        (Some(since), Some(until)) => format!("Period: {} to {} (exclusive)\n", since, until),
        (Some(since), None) => format!("Period: {} onward\n", since),
        (None, Some(until)) => format!("Period: up to {} (exclusive)\n", until),
        (None, None) => String::new(),
    }
}

/// Runs the `balance`/`trial-balance` subcommand: loads `file`, prints its trial balance, and
/// fails (nonzero exit) if any commodity section doesn't foot.
fn run_balance(file: &std::path::Path, date_range: &DateRangeArgs) -> Result<(), String> {
    let transactions = load_and_filter(file, date_range)?;
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
fn run_balance_sheet(file: &std::path::Path, date_range: &DateRangeArgs) -> Result<(), String> {
    let transactions = load_and_filter(file, date_range)?;
    let ledger = Ledger::from_transactions(transactions);
    let report = balance_sheet::build(&ledger);
    print!("{}", report.render());
    if !report.is_balanced() {
        return Err("balance sheet does not balance".to_string());
    }
    Ok(())
}

/// Runs the `income-statement`/`is` subcommand: loads `file` and prints its income statement.
/// There's no accounting-equation check here (unlike `balance`/`balance-sheet`) — an income
/// statement doesn't have one on its own, so this only fails if the journal itself fails to load.
fn run_income_statement(file: &std::path::Path, date_range: &DateRangeArgs) -> Result<(), String> {
    let transactions = load_and_filter(file, date_range)?;
    let ledger = Ledger::from_transactions(transactions);
    let report = income_statement::build(&ledger);
    print!("{}", report.render());
    Ok(())
}

/// Runs the `check` subcommand: loads and balance-validates `file`, printing nothing on
/// success. Not scoped by `--since`/`--until` — see the flag's doc comment in `cli.rs`.
fn run_check(file: &std::path::Path) -> Result<(), String> {
    load_journal(file).map_err(|e| e.to_string())?;
    Ok(())
}

/// Runs the `clear` subcommand: loads `file` and prints a clearing-group analysis for each
/// named account (see [`ferro_ledger::reports::clearing`]).
fn run_clear(file: &std::path::Path, accounts: &[String], date_range: &DateRangeArgs) -> Result<(), String> {
    let transactions = load_and_filter(file, date_range)?;
    let ledger = Ledger::from_transactions(transactions);
    let accounts: Vec<Account> = accounts.iter().map(Account::new).collect();
    let reports = clearing::analyze(&ledger, &accounts);
    print!("{}", clearing::render(&reports));
    Ok(())
}
