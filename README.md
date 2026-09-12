# ferro_ledger

A text-based, command-line general ledger, written in Rust.

- **No database.** Your books are a plain-text journal file — git-friendly, diffable,
  editable in any text editor. Every run reads the file fresh and recomputes everything.
- **hledger-compatible journal syntax.** ferro_ledger reads a documented subset of
  [hledger](https://hledger.org)'s journal format, so a `.journal` file written for
  ferro_ledger is also valid input to `hledger`, as far as that subset goes.
- **Double-entry, always.** Every transaction's postings must sum to zero (per commodity) —
  enforced when the journal is loaded, not deferred to report time.
- **Trial balance is the primary report**, plus a balance sheet, an income statement, and a
  clearing/suspense-account analyzer for tracking groups of postings (e.g. a payment recorded
  now, settled later) until they net to zero.
- **Every report can be scoped to a date range** with `--since`/`--until`, so you can ask "what
  did January look like" without editing the journal.

## Why

Most personal/small-business ledger tools are either a full accounting *application* (a
database, a UI, an install) or a big, general-purpose tool like hledger itself. ferro_ledger
is the minimal middle ground: a single small CLI binary, your journal is a text file you can
read and edit by hand, and the reports it produces (trial balance, balance sheet, income
statement, clearing-account status) are the ones you reach for constantly when keeping books
this way.

## Installing / building

Requires a Rust toolchain ([rustup.rs](https://rustup.rs)).

```sh
git clone git@github.com:johnhealio/ferro_ledger.git
cd ferro_ledger
cargo build --release
./target/release/ferro_ledger --help
```

Or run straight from source with `cargo run --`.

## Quick start

```sh
cargo run -- balance examples/sample.journal
```

```
Commodity: USD
Account                            Debit          Credit
--------------------------------------------------------
Assets:Bank:Checking             4204.80
Assets:Clearing:Payments          320.00
Equity:OpeningBalance                            5000.00
Expenses:Groceries                 45.20
Expenses:Payroll:Gross           2000.00
Income:Sales                                     1070.00
Liabilities:PayrollTaxes                          500.00
--------------------------------------------------------
TOTAL                            6570.00         6570.00
```

```sh
cargo run -- balance-sheet examples/sample.journal
```

```
Commodity: USD
ASSETS
  Assets:Bank:Checking             4204.80
  Assets:Clearing:Payments          320.00
  Total Assets                     4524.80

LIABILITIES
  Liabilities:PayrollTaxes          500.00
  Total Liabilities                 500.00

EQUITY
  Equity:OpeningBalance         5000.00
  Net Income (unclosed)         -975.20
  Total Equity                  4024.80

Assets (4524.80) = Liabilities + Equity (4524.80)
```

`Net Income (unclosed)` is a computed line, not a real account — Income and Expenses haven't
been closed into Equity, so ferro_ledger nets them itself so the sheet still balances. See
[`docs/BALANCE_SHEET.md`](docs/BALANCE_SHEET.md).

```sh
cargo run -- income-statement examples/sample.journal
```

```
Commodity: USD
REVENUE
  Income:Sales          1070.00
  Total Revenue         1070.00

EXPENSES
  Expenses:Groceries               45.20
  Expenses:Payroll:Gross         2000.00
  Total Expenses                 2045.20

NET LOSS: -975.20
```

This net figure always matches the balance sheet's `Net Income (unclosed)` row above — both are
computed from the same Revenue/Expense balances. See
[`docs/INCOME_STATEMENT.md`](docs/INCOME_STATEMENT.md).

```sh
cargo run -- clear examples/sample.journal --account Assets:Clearing:Payments
```

```
Clearing account: Assets:Clearing:Payments
  1 cleared group(s), 1 outstanding group(s)
  [OUTSTANDING] match:INV-2051  net 320.00 USD (1 posting(s))
      2024-01-12  Invoice INV-2051 recorded                         320.00 USD  (examples/sample.journal:21)
  [CLEARED] match:INV-2044  net 0.00 USD (2 posting(s))
      2024-01-05  Invoice INV-2044 recorded                         750.00 USD  (examples/sample.journal:13)
      2024-01-09  Bank settlement for INV-2044                     -750.00 USD  (examples/sample.journal:18)
```

## Commands

| Command | What it does |
|---|---|
| `ferro_ledger balance <FILE>` (alias `trial-balance`) | Prints a trial balance: one row per account, debit/credit columns, one section per commodity. Exits non-zero if a section doesn't foot. |
| `ferro_ledger balance-sheet <FILE>` (alias `bs`) | Prints a balance sheet: Assets, Liabilities, and Equity, classified from each account's top-level segment. Exits non-zero if it doesn't balance. |
| `ferro_ledger income-statement <FILE>` (alias `is`) | Prints an income statement: Revenue, Expenses, and net income/loss. |
| `ferro_ledger clear <FILE> --account <ACCOUNT>...` | Groups a clearing/suspense account's postings by matching key and reports which groups have cleared (net to zero) vs. are still outstanding. Repeat `--account` for multiple accounts. |
| `ferro_ledger check <FILE>` | Parses and balance-validates the journal only. Prints nothing and exits 0 on success — useful in CI/pre-commit. |

`balance`, `balance-sheet`, `income-statement`, and `clear` all also accept `--since <DATE>`
and/or `--until <DATE>` (inclusive start, exclusive end — see
[`docs/DATE_RANGE.md`](docs/DATE_RANGE.md)) to scope the report to a date window:

```sh
cargo run -- balance examples/sample.journal --since 2024-01-20
```

```
Period: 2024-01-20 onward

Commodity: USD
Account                            Debit          Credit
--------------------------------------------------------
Assets:Bank:Checking                             1500.00
Expenses:Payroll:Gross           2000.00
Liabilities:PayrollTaxes                          500.00
--------------------------------------------------------
TOTAL                            2000.00         2000.00
```

`check` deliberately has no date-range flags — it validates that every transaction balances,
which isn't a date-scoped property.

## Writing a journal

A transaction looks like this:

```
2024-01-15 * Payment received
    Assets:Bank:Checking          100.00 USD
    Income:Sales                 -100.00 USD
```

The last posting in a transaction may omit its amount — it's inferred as whatever balances the
others:

```
2024-01-16 Grocery shopping
    Expenses:Groceries             45.00 USD
    Assets:Bank:Checking
```

Full syntax reference: [`docs/JOURNAL_FORMAT.md`](docs/JOURNAL_FORMAT.md).

### Clearing / suspense accounts

Route a transaction through a clearing account instead of hitting the final account directly
when its other half won't arrive until later (a bank settlement, a payroll disbursement, an
inter-company transfer). Tag both legs with a shared `match:` value so `ferro_ledger clear` can
pair them up:

```
2024-01-05 * Invoice recorded
    Assets:Clearing:Payments      500.00 USD  ; match:INV-2044
    Income:Sales                 -500.00 USD

2024-01-09 * Bank settlement
    Assets:Bank:Checking           500.00 USD
    Assets:Clearing:Payments      -500.00 USD  ; match:INV-2044
```

Without a `match:` tag, postings of equal absolute amount in the same clearing account are
paired as a fallback — but that's coarse (two unrelated postings of the same size will be
mismatched), so tag anything you care about reconciling correctly. Full design:
[`docs/CLEARING_ACCOUNTS.md`](docs/CLEARING_ACCOUNTS.md).

## Documentation

- [`CLAUDE.md`](CLAUDE.md) — project conventions and orientation.
- [`docs/PLANNING.md`](docs/PLANNING.md) — phased build plan and status.
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — module layout and data flow.
- [`docs/JOURNAL_FORMAT.md`](docs/JOURNAL_FORMAT.md) — supported journal syntax.
- [`docs/BALANCE_SHEET.md`](docs/BALANCE_SHEET.md) — balance sheet: account classification, net
  income folding.
- [`docs/INCOME_STATEMENT.md`](docs/INCOME_STATEMENT.md) — income statement: Revenue/Expenses,
  net income.
- [`docs/DATE_RANGE.md`](docs/DATE_RANGE.md) — `--since`/`--until` date-range scoping.
- [`docs/CLEARING_ACCOUNTS.md`](docs/CLEARING_ACCOUNTS.md) — clearing/suspense account design.

API docs (rustdoc) can be built locally:

```sh
cargo doc --no-deps --open
```

## Development

```sh
cargo test              # unit + integration tests
cargo clippy --all-targets
cargo fmt
```

## Status / roadmap

v1 covers: journal parsing, double-entry validation, trial balance, balance sheet, income
statement, clearing-account analysis, and date-range filtering. Not yet implemented:
multi-currency conversion, a journal-rewriting `clear --mark` mode, and a `--period` shorthand
for date ranges. See [`docs/PLANNING.md`](docs/PLANNING.md) for details.

## License

No license file has been added yet — until one is, all rights are reserved by the author.
