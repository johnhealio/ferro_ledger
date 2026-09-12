# Planning

## Goal

A CLI general ledger, in Rust, that:

1. Reads a plain-text journal file in a subset of hledger's format.
2. Validates double-entry balancing.
3. Produces a **trial balance** as its primary report.
4. Supports **clearing/suspense accounts**: postings routed through a designated clearing account
   are grouped (by a matching tag) and reported as "cleared" once a group nets to zero, or
   "outstanding" while it doesn't.

No database. The journal text file(s) are the only persistent store, always re-read and
re-computed — this mirrors hledger/ledger-cli exactly.

## Phases

### Phase 0 — Project scaffolding (this session)
- [x] `cargo init`, repo layout, `.gitignore`.
- [x] `CLAUDE.md` + `docs/` planning set (this file, architecture, journal format, clearing design).

### Phase 1 — Core data model & parser
- [x] `Amount` (decimal quantity + commodity/currency symbol).
- [x] `Posting` (account, amount — possibly elided, comment, inline tags).
- [x] `Transaction` (date, optional secondary date, status mark, code, description, comment,
      tags, postings).
- [x] Hand-written journal parser for the subset in `docs/JOURNAL_FORMAT.md`:
      transaction headers, postings, elided amount inference, comments, tags, blank-line
      transaction separation, `;`/`#` full-line comments, `account`/`commodity`/`include`
      directives (parsed and at minimum not-fatal if unsupported further).
- [x] Balance validation: each transaction's postings must sum to zero per commodity (after
      elision inference); parser errors carry file/line context.

### Phase 2 — Ledger aggregation & trial balance report
- [x] `Ledger`: loads one or more journal files, flattens to a `Vec<Transaction>`, indexes
      postings by account.
- [x] Account tree semantics: `Assets:Bank:Checking` rolls up into `Assets:Bank` and `Assets`
      for reporting subtotals, matching hledger's colon-hierarchy convention (`subtree_balance`;
      the trial balance report itself lists leaf accounts, per real trial-balance convention).
- [x] Trial balance report: one row per account with debit/credit columns, total row, and a
      hard check that total debits == total credits (should always hold if parsing validated
      balance — the report re-derives it independently as a cross-check).
- [x] CLI: `ferro_ledger balance <journal>` (alias `trial-balance`), plain-text table output.

### Phase 3 — Clearing / suspense accounts
- [x] Design doc (`docs/CLEARING_ACCOUNTS.md`) settled: matching key = `match:` tag on a
      posting, falling back to (account, abs(amount), commodity) when no tag is present.
- [x] `clear` report: given one or more clearing-account names, group that account's postings by
      matching key, report each group's net (cleared if zero, outstanding otherwise) and its
      member transactions.
- [x] CLI: `ferro_ledger clear <journal> --account Assets:Clearing:...`.

### Phase 4 — Polish / hledger compatibility checks
- [x] Sample journal in `examples/sample.journal`, exercising opening balances, an elided
      posting, a two-leg tag-matched clearing group, an outstanding one, and a three-leg payroll
      clearing group. `hledger` itself wasn't installed in this environment to cross-check
      byte-for-byte, so compatibility was verified by careful reading of hledger's documented
      journal grammar rather than running the real binary — worth a manual `hledger -f
      examples/sample.journal print` diff later if/when `hledger` is available.
- [x] Integration tests: `tests/parser_tests.rs` (file I/O, `include` resolution, circular
      include detection), `tests/trial_balance_tests.rs`, `tests/clearing_tests.rs` (tag
      matching, fallback matching, subtree rollup into clearing sub-accounts).
- [x] Usage instructions kept in this docs set (see the Quick start below) and in `README.md`
      for GitHub visitors.
- [x] rustdoc comments on every public item (`cargo doc --no-deps` clean, verified with
      `-W missing_docs`).

### Phase 5 — Balance sheet report
- [x] Design doc (`docs/BALANCE_SHEET.md`) settled: accounts classified by top-level segment
      name (`src/account_types.rs`, case-insensitive, singular/plural); Liability and Equity
      rows sign-flipped so a normal balance displays positive; unclosed Revenue/Expense activity
      folded into Equity as a synthetic "Net Income (unclosed)" row so the sheet balances
      without requiring period-end closing entries; accounts with an unrecognized top-level
      segment are excluded from every section and listed separately rather than guessed at.
- [x] `balance_sheet::build`/`render`, one sheet per commodity, with an
      Assets == Liabilities + Equity check per sheet.
- [x] CLI: `ferro_ledger balance-sheet <journal>` (alias `bs`).
- [x] Integration tests: `tests/balance_sheet_tests.rs` (balances with no P&L activity, net
      income folding, sign display, unclassified-account exclusion).

### Phase 6 — Income statement report
- [x] Design doc (`docs/INCOME_STATEMENT.md`) settled: reuses `src/account_types.rs`'s
      Revenue/Expense classification; Revenue rows sign-flipped to display positive; a net
      income/loss bottom line (Revenue − Expenses); Asset/Liability/Equity accounts are out of
      scope (not flagged as an anomaly), while a genuinely unrecognized top-level segment is
      still listed, same as the balance sheet does.
- [x] `income_statement::build`/`render`, one statement per commodity.
- [x] CLI: `ferro_ledger income-statement <journal>` (alias `is`).
- [x] Integration tests: `tests/income_statement_tests.rs`, including a direct cross-check that
      its net income always equals the balance sheet's folded-in "Net Income (unclosed)" row —
      both are computed from the same Revenue/Expense balances.
- [x] Known v1 limitation, resolved in Phase 7 below: date-range filtering.

### Phase 7 — Date-range filtering
- [x] Design doc (`docs/DATE_RANGE.md`) settled: `--since` (inclusive) / `--until` (exclusive)
      flags, mirroring hledger's `-b`/`-e` under clearer names; `src/date_range.rs`'s
      `DateRange::filter` applied to a journal's `Vec<Transaction>` in `main.rs`, strictly
      *before* a `Ledger` is built — every report module stays exactly as date-unaware as
      before, filtering is entirely main.rs's/date_range.rs's concern.
- [x] `parser::parse_date_str` made public and reused by the CLI flags, so a date string means
      the same thing on the command line as it does inside a journal file.
- [x] `#[command(flatten)] date_range: DateRangeArgs` on every report subcommand
      (`balance`/`balance-sheet`/`income-statement`/`clear`); `check` deliberately excluded (see
      `docs/DATE_RANGE.md`, "`check` is intentionally not scoped").
- [x] A one-line "Period: ..." header printed before scoped report output, so it's visually
      obvious a result is filtered rather than covering the whole journal.
- [x] Integration tests: `tests/date_range_tests.rs` (library-level `DateRange::filter`
      semantics) and `tests/cli_date_range_tests.rs` (end-to-end against the actual compiled
      binary via `CARGO_BIN_EXE_ferro_ledger`, including the invalid-date error path and that
      `check` rejects the flags).

## Quick start

```sh
cargo run -- balance examples/sample.journal          # trial balance (alias: trial-balance)
cargo run -- balance-sheet examples/sample.journal     # balance sheet (alias: bs)
cargo run -- income-statement examples/sample.journal  # income statement (alias: is)
cargo run -- clear examples/sample.journal --account Assets:Clearing:Payments --account Assets:Clearing:Payroll
cargo run -- balance examples/sample.journal --since 2024-01-01 --until 2024-02-01  # date-scoped
cargo run -- check examples/sample.journal             # parse + balance-validate only
cargo test                                             # unit + integration tests
```

## Explicitly out of scope for v1

- Multi-currency conversion / exchange rates (`P` price directives) — parsed-and-ignored at most.
- Budgets, forecasting, periodic transactions.
- A `--period` shorthand expression (hledger's `-p "jan-mar 2024"` style) that expands to an
  equivalent `--since`/`--until` pair — the two explicit flags cover the same ground with more
  typing. See "Not implemented: a `--period` shorthand" in `docs/DATE_RANGE.md`.
- A real `close` (period-end closing entry) command — the balance sheet's net-income folding
  (`docs/BALANCE_SHEET.md`) is a computed stand-in, not a replacement.
- `account`-directive account types (`account Assets:Bank ; type:A`) — the balance sheet's and
  income statement's classification is name-based only (`src/account_types.rs`); explicit type
  declarations would be a natural follow-up if name-based classification proves too fragile in
  practice.
- Auto-rewriting the journal file to mark transactions `*` cleared — the `clear` report is
  read-only in v1; see `docs/CLEARING_ACCOUNTS.md` for why and what v2 would need.
- A TUI — this is a plain argument-driven CLI (`clap`) that prints reports and exits.

## Open decisions log

- **Matching key for clearing groups**: use a `match:<id>` tag rather than inferring solely from
  amount, because amount-only matching breaks as soon as two unrelated postings of the same
  value land in the same clearing account in the same period. Tag-based matching is explicit and
  mirrors how real bookkeepers use reference/cheque numbers. Amount+commodity fallback exists so
  small example journals don't require tagging everything. See `docs/CLEARING_ACCOUNTS.md`.
- **Decimal library**: `rust_decimal` over `f64`, non-negotiable for money.
- **Parser approach**: hand-rolled, not `nom`/`pest` — the format is small and line-structured;
  a hand-rolled parser gives us precise, hledger-shaped error messages more easily.
- **Balance sheet folds net income into Equity** rather than shipping a report that's routinely
  wrong (or requiring the user to write closing entries first). See `docs/BALANCE_SHEET.md`.
  Accounts the classifier can't place are excluded and listed explicitly rather than guessed at,
  for the same "don't silently do the wrong thing" reason `docs/CLEARING_ACCOUNTS.md` gives for
  flagging mixed-commodity clearing groups as an anomaly instead of quietly netting them.
- **Date-range filtering happens on `Vec<Transaction>`, before the `Ledger` exists**, not as a
  parameter threaded through every report's `build()`. This keeps every report module exactly as
  simple and date-unaware as it was before the feature existed, at the cost of `main.rs` doing
  slightly more work per subcommand. See `docs/DATE_RANGE.md`.
- **`--since`/`--until` over a single `--period` expression**: two explicit flags are simpler to
  implement and to reason about than parsing hledger-style period expressions, at the cost of
  more typing for the common cases a period expression would shorten. Revisit if that friction
  turns out to matter in practice.

## Status

Phases 0–7 are complete: parser, ledger, trial balance, balance sheet, income statement,
clearing-account analysis, and date-range filtering are all implemented and tested (`cargo
test`: 39 passed; `cargo clippy --all-targets`: clean; `cargo doc --no-deps`: clean). Next
natural increments: multi-currency conversion, a journal-rewriting `clear --mark` mode (see
"Future work" in `docs/CLEARING_ACCOUNTS.md`), and possibly a `--period` shorthand if
`--since`/`--until` proves too verbose in practice.
