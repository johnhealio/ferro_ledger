# Architecture

## Crate layout

```
src/
  main.rs           CLI entry point: arg parsing (clap), date-range resolution + filtering,
                     dispatch to commands, error printing
  cli.rs            clap Parser/Subcommand definitions, incl. shared DateRangeArgs (flattened
                     into every report subcommand)
  model.rs          Core types: Amount, Account (newtype over String), Posting, Transaction,
                     Status, Tag
  parser.rs         Hand-written journal text -> Vec<Transaction> parser (+ parser unit tests);
                     also exposes parse_date_str, reused by the CLI's date-range flags
  journal.rs        Loads a journal file (and `include`d files) into a Vec<Transaction>
  date_range.rs     DateRange: the --since/--until window applied to transactions before a
                     Ledger is built (docs/DATE_RANGE.md)
  ledger.rs         Ledger: owns transactions, indexes postings by account, exposes balance
                     queries (leaf balance, subtree balance via colon-hierarchy rollup)
  account_types.rs  Name-based Asset/Liability/Equity/Revenue/Expense classification, used by
                     the balance sheet report (docs/BALANCE_SHEET.md)
  reports/
    mod.rs
    trial_balance.rs  TrialBalance report: builds rows from a Ledger, renders as text table
    balance_sheet.rs  BalanceSheet report: Assets/Liabilities/Equity via account_types.rs,
                       folds unclosed net income into Equity (docs/BALANCE_SHEET.md)
    income_statement.rs  IncomeStatement report: Revenue/Expenses via account_types.rs, ending
                       in a net income/loss line (docs/INCOME_STATEMENT.md)
    clearing.rs       Clearing-group analysis: groups postings in named accounts, nets them,
                       renders cleared/outstanding report
examples/
  *.journal         Sample hledger-compatible journals used as manual smoke tests
tests/
  parser_tests.rs
  trial_balance_tests.rs
  balance_sheet_tests.rs
  income_statement_tests.rs
  clearing_tests.rs
  date_range_tests.rs         library-level DateRange::filter tests
  cli_date_range_tests.rs     end-to-end tests against the built binary (CARGO_BIN_EXE_*)
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
   journal::load()                     DateRange::filter()   (--since/--until, no-op if unset)
   (resolves `include`)                        │
                                                ▼
                                     ledger::Ledger::from_transactions()
                                                │
                                                ▼
          reports::trial_balance / reports::balance_sheet / reports::income_statement / reports::clearing
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

## Date-range filtering (`date_range.rs`)

`DateRange { since: Option<NaiveDate>, until: Option<NaiveDate> }` with
`filter(Vec<Transaction>) -> Vec<Transaction>` (inclusive `since`, exclusive `until`; see
`docs/DATE_RANGE.md`) is applied in `main.rs` between `journal::load_journal` and
`Ledger::from_transactions` — a `Ledger` never knows whether it was built from the whole journal
or a scoped window, which is exactly why every report module stays date-unaware. The CLI's
`--since`/`--until` strings are parsed by `parser::parse_date_str`, the same function the
journal parser itself uses for transaction dates, so a date string means the same thing on the
command line as it does in the journal.

## Account classification (`account_types.rs`)

`classify(&Account) -> Option<AccountType>` maps an account's top-level segment
(case-insensitively, singular or plural: `assets`, `liabilities`, `equity`, `income`/`revenue`,
`expenses`) to an `AccountType`, or `None` if it doesn't match any of those. This is a name-based
heuristic (v1 has no `account`-directive type declarations — see `docs/JOURNAL_FORMAT.md`), used
today only by the balance sheet report but factored out so a future income statement report can
reuse the same Revenue/Expense classification.

## Reports (`reports/`)

Reports are pure functions of a `&Ledger` (or `&[Transaction]`) that return a data structure,
which a separate render step turns into text. Keeping "compute the report" and "print the report"
separate means report logic is unit-testable without stdout capture.

- `trial_balance::build(&Ledger) -> TrialBalance` / `TrialBalance::render() -> String`
- `balance_sheet::build(&Ledger) -> BalanceSheetReport` / `BalanceSheetReport::render() -> String`
  — see `docs/BALANCE_SHEET.md` for the account-classification and net-income-folding design.
- `income_statement::build(&Ledger) -> IncomeStatementReport` /
  `IncomeStatementReport::render() -> String` — see `docs/INCOME_STATEMENT.md`.
- `clearing::analyze(&Ledger, accounts: &[Account]) -> Vec<ClearingAccountReport>` /
  `clearing::render(&[ClearingAccountReport]) -> String`

## CLI (`cli.rs`, `main.rs`)

`clap` derive-based subcommands, all but `check` also taking the flattened `DateRangeArgs`
(`--since`/`--until`, see `docs/DATE_RANGE.md`):

- `ferro_ledger balance <FILE> [--since DATE] [--until DATE]` (alias `trial-balance`) — trial
  balance report.
- `ferro_ledger balance-sheet <FILE> [--since DATE] [--until DATE]` (alias `bs`) — balance sheet
  report.
- `ferro_ledger income-statement <FILE> [--since DATE] [--until DATE]` (alias `is`) — income
  statement report.
- `ferro_ledger clear <FILE> --account <ACCOUNT>... [--since DATE] [--until DATE]` —
  clearing-account matching report.
- `ferro_ledger check <FILE>` — parse + balance-validate only, exit non-zero on any error
  (useful as a pre-commit/CI check on the journal, same spirit as `hledger check`); deliberately
  has no date-range flags — see `docs/DATE_RANGE.md`.

`main.rs` is intentionally thin: parse args, resolve `--since`/`--until` into a `DateRange` and
filter, call into `journal`/`ledger`/`reports`, format errors for a human, set exit code. All
real logic is in the library modules so it's testable without spawning the binary — except the
CLI wiring itself (clap's `#[command(flatten)]`, argument parsing end to end), which
`tests/cli_date_range_tests.rs` tests by running the actual compiled binary.
