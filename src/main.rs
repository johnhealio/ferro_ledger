use clap::Parser;
use ferro_ledger::cli::{Cli, Command};
use ferro_ledger::ledger::Ledger;
use ferro_ledger::model::Account;
use ferro_ledger::reports::{clearing, trial_balance};
use ferro_ledger::journal::load_journal;

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Balance { file } => run_balance(&file),
        Command::Check { file } => run_check(&file),
        Command::Clear { file, accounts } => run_clear(&file, &accounts),
    };

    if let Err(message) = result {
        eprintln!("error: {}", message);
        std::process::exit(1);
    }
}

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

fn run_check(file: &std::path::Path) -> Result<(), String> {
    load_journal(file).map_err(|e| e.to_string())?;
    Ok(())
}

fn run_clear(file: &std::path::Path, accounts: &[String]) -> Result<(), String> {
    let transactions = load_journal(file).map_err(|e| e.to_string())?;
    let ledger = Ledger::from_transactions(transactions);
    let accounts: Vec<Account> = accounts.iter().map(Account::new).collect();
    let reports = clearing::analyze(&ledger, &accounts);
    print!("{}", clearing::render(&reports));
    Ok(())
}
