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

## Quick start

```sh
cargo run -- balance examples/sample.journal        # trial balance (alias: trial-balance)
cargo run -- balance-sheet examples/sample.journal   # balance sheet (alias: bs)
cargo run -- clear examples/sample.journal --account Assets:Clearing:Payments --account Assets:Clearing:Payroll
cargo run -- check examples/sample.journal           # parse + balance-validate only
cargo test                                           # unit + integration tests
```

## Explicitly out of scope for v1

- Multi-currency conversion / exchange rates (`P` price directives) — parsed-and-ignored at most.
- Budgets, forecasting, periodic transactions.
- An income statement / P&L report, and a real `close` (period-end closing entry) command —
  the balance sheet's net-income folding (`docs/BALANCE_SHEET.md`) is a computed stand-in for
  the latter, not a replacement for either.
- `account`-directive account types (`account Assets:Bank ; type:A`) — the balance sheet's
  classification is name-based only (`src/account_types.rs`); explicit type declarations would
  be a natural follow-up if name-based classification proves too fragile in practice.
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

## Status

Phases 0–5 are complete: parser, ledger, trial balance report, clearing-account analysis, and
balance sheet report are all implemented and tested (`cargo test`: 25 passed; `cargo clippy
--all-targets`: clean; `cargo doc --no-deps`: clean). Next natural increments: an income
statement report (would reuse `account_types.rs`'s Revenue/Expense classification), multi-currency
conversion, and a journal-rewriting `clear --mark` mode (see "Future work" in
`docs/CLEARING_ACCOUNTS.md`).
