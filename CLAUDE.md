# CLAUDE.md

This file orients Claude (and future contributors) working in this repository.

## What this is

**ferro_ledger** is a text-based, command-line general ledger written in Rust.
It follows the philosophy of [hledger](https://hledger.org)/[ledger-cli](https://ledger-cli.org):

- The books are a **plain text journal file** (no database). Git-friendly, diffable, human-editable.
- Journal syntax is a **compatible subset of hledger's format** — see [docs/JOURNAL_FORMAT.md](docs/JOURNAL_FORMAT.md).
  A `.journal` file written for ferro_ledger should also be valid input to `hledger`, as far as the
  supported subset goes. We never invent syntax hledger wouldn't parse.
- Double-entry bookkeeping: every transaction's postings must sum to zero (per commodity).
- The primary report is the **trial balance** (`ferro_ledger balance` / `trial-balance`).
- Some balance sheet accounts are **clearing / suspense accounts**: transactions routed through them
  are expected to arrive in offsetting groups that net to zero and "clear". See
  [docs/CLEARING_ACCOUNTS.md](docs/CLEARING_ACCOUNTS.md).

Read [docs/PLANNING.md](docs/PLANNING.md) for project phasing/status and
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for module layout before making structural changes.

## Documentation map

| File | Purpose |
|---|---|
| `CLAUDE.md` | This file — orientation and conventions |
| `docs/PLANNING.md` | Phased build plan, current status, open decisions |
| `docs/ARCHITECTURE.md` | Crate/module layout, data model, data flow |
| `docs/JOURNAL_FORMAT.md` | Supported journal syntax (hledger-compatible subset) |
| `docs/CLEARING_ACCOUNTS.md` | Clearing/suspense account design: matching & clearing semantics |
| `docs/BALANCE_SHEET.md` | Balance sheet report: account-type classification, net-income folding |
| `docs/INCOME_STATEMENT.md` | Income statement report: Revenue/Expenses, net income, no date filtering yet |

## Conventions for this codebase

- **No database, ever.** Persistence is the journal text file(s) on disk, read fresh each run.
  Any "state" (e.g. which transactions have cleared) is either recomputed from the journal on
  every run, or written back into the journal as plain text (e.g. a status marker), never into a
  side database.
- **Money is `rust_decimal::Decimal`.** Never use `f32`/`f64` for amounts — floating point makes
  balances that don't foot to zero.
- **Errors:** library code returns `anyhow::Result` / `thiserror` error enums; the CLI binary is
  the only place that prints errors and sets the process exit code.
- **Parsing is hand-written**, not a external parsing library or a generated grammar — the
  supported hledger subset is small and line-oriented enough that a hand-rolled line/column
  parser is clearer than a PEG for this project's needs. Keep `src/parser.rs` the single source
  of truth for journal syntax; `docs/JOURNAL_FORMAT.md` documents what it accepts.
- **Compatibility discipline:** before adding a journal syntax feature, check whether real hledger
  accepts the same syntax the same way. If a sample file would parse differently in hledger than in
  ferro_ledger, that's a bug (or an explicitly documented, deliberate subset limitation).
- Keep example journals in `examples/` runnable via the CLI as manual smoke tests.
- Tests live under `tests/` (integration, one file per feature area) and inline `#[cfg(test)]`
  modules for pure unit logic (e.g. parser token-level tests).

## Working agreements

- Update `docs/PLANNING.md`'s status section whenever a phase completes or the plan changes —
  it's the living record of what's done vs. planned, not a one-time design doc.
- Don't add a database, ORM, or persistent cache under any circumstance — that's the one
  non-negotiable architectural constraint of this project.
