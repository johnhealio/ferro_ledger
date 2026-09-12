# Architecture

## Crate layout

```
src/
  main.rs           CLI entry point: arg parsing (clap), dispatch to commands, error printing
  cli.rs            clap Parser/Subcommand definitions
  model.rs          Core types: Amount, Account (newtype over String), Posting, Transaction,
                     Status, Tag
  parser.rs         Hand-written journal text -> Vec<Transaction> parser (+ parser unit tests)
  journal.rs        Loads a journal file (and `include`d files) into a Vec<Transaction>
  ledger.rs         Ledger: owns transactions, indexes postings by account, exposes balance
                     queries (leaf balance, subtree balance via colon-hierarchy rollup)
  reports/
    mod.rs
    trial_balance.rs  TrialBalance report: builds rows from a Ledger, renders as text table
    clearing.rs       Clearing-group analysis: groups postings in named accounts, nets them,
                       renders cleared/outstanding report
examples/
  *.journal         Sample hledger-compatible journals used as manual smoke tests
tests/
  parser_tests.rs
  trial_balance_tests.rs
  clearing_tests.rs
```

## Data flow

```
journal file(s) on disk
        │  read_to_string
        ▼
   parser::parse()  ──────────────►  Vec<Transaction>   (pure data, no I/O)
        │                                   │
        │ (balance-check each txn)          │
        ▼                                   ▼
   journal::load()                     ledger::Ledger::from_transactions()
   (resolves `include`)                        │
                                                ▼
                                   reports::trial_balance / reports::clearing
                                                │
                                                ▼
                                          CLI prints table
```

Everything downstream of `parser::parse` is pure/deterministic given the `Vec<Transaction>` — no
report holds a file handle or mutates the journal. The journal file is the single source of
truth; every run re-parses it from scratch. This is the direct consequence of the "no database"
rule in `CLAUDE.md`: there is no cache to invalidate, no schema to migrate, no on-disk state that
can drift from the text file.

## Core types (`model.rs`)

- `Account(String)` — newtype so account-hierarchy logic (colon-split, `is_under`, parent chain)
  lives in one place instead of being re-derived on raw `String`s everywhere.
- `Amount { quantity: rust_decimal::Decimal, commodity: String }` — commodity is just its symbol
  (`"USD"`, `"$"`, `""` for no-symbol numbers); v1 does no cross-commodity conversion, so amounts
  in different commodities are simply reported as separate columns/sections and never summed
  together.
- `Posting { account: Account, amount: Option<Amount>, comment: Option<String>, tags: Vec<Tag>, status: Option<Status> }`
  — `amount` is `None` for an elided posting (hledger allows the last posting in a transaction to
  omit its amount; the parser infers it as "whatever balances the transaction").
- `Transaction { date: NaiveDate, secondary_date: Option<NaiveDate>, status: Status, code: Option<String>, description: String, comment: Option<String>, tags: Vec<Tag>, postings: Vec<Posting>, source: SourcePos }`
  — `source` (file + line) is carried through purely for good error/report messages.
- `Status` — `Unmarked | Pending | Cleared`, mirroring hledger's `(none)/!/*`  transaction-status
  markers. This is the *transaction*-level status hledger defines; it is a different concept from
  the ledger-level "clearing account" reconciliation in `docs/CLEARING_ACCOUNTS.md` — v1 does not
  conflate the two (see that doc's "Relationship to hledger's `*`/`!` status" section).
- `Tag(String, Option<String>)` — parsed out of comments (`; key:value, key2:value2`), per
  hledger's tag syntax.

## Ledger (`ledger.rs`)

`Ledger::from_transactions(Vec<Transaction>) -> Ledger` builds:

- A flat list of `(Transaction index, Posting index)` per account (leaf account, exact string).
- A balance-lookup that, given an `Account`, sums postings whose account is *that account or any
  descendant* (colon-prefix match) — this is what makes `Assets` show the rolled-up total of
  `Assets:Bank:Checking` + `Assets:Bank:Savings` + ... in the trial balance, matching hledger.

No caching layer beyond this in-memory struct for the lifetime of one CLI invocation — it is
rebuilt from the parsed transactions every run, per the no-database rule.

## Reports (`reports/`)

Reports are pure functions of a `&Ledger` (or `&[Transaction]`) that return a data structure,
which a separate render step turns into text. Keeping "compute the report" and "print the report"
separate means report logic is unit-testable without stdout capture.

- `trial_balance::build(&Ledger) -> TrialBalance` / `TrialBalance::render() -> String`
- `clearing::analyze(&Ledger, accounts: &[Account]) -> ClearingAnalysis` /
  `ClearingAnalysis::render() -> String`

## CLI (`cli.rs`, `main.rs`)

`clap` derive-based subcommands:

- `ferro_ledger balance <FILE> [--flat]` (alias `trial-balance`) — trial balance report.
- `ferro_ledger clear <FILE> --account <ACCOUNT>...` — clearing-account matching report.
- `ferro_ledger check <FILE>` — parse + balance-validate only, exit non-zero on any error
  (useful as a pre-commit/CI check on the journal, same spirit as `hledger check`).

`main.rs` is intentionally thin: parse args, call into `journal`/`ledger`/`reports`, format
errors for a human, set exit code. All real logic is in the library modules so it's testable
without spawning the binary.
