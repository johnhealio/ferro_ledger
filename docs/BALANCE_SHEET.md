# Balance sheet report

`ferro_ledger balance-sheet <journal>` (alias `bs`) prints Assets, Liabilities, and Equity, and
checks the fundamental accounting equation: **Assets = Liabilities + Equity**.

## Classifying accounts

ferro_ledger has no formal "account type" declaration in v1 (see "Directives" in
`docs/JOURNAL_FORMAT.md` — `account` is accepted but carries no type). Instead, the balance
sheet classifies each account by its **top-level segment**, case-insensitively, singular or
plural — the same heuristic hledger falls back to when accounts aren't given explicit types:

| Top-level segment (any case) | Classified as |
|---|---|
| `asset`, `assets` | Asset |
| `liability`, `liabilities` | Liability |
| `equity` | Equity |
| `income`, `revenue`, `revenues` | Revenue |
| `expense`, `expenses` | Expense |
| anything else | *unclassified* — see below |

This lives in `src/account_types.rs` as `classify(&Account) -> Option<AccountType>`, so other
reports can reuse it later (an income statement would use the same Revenue/Expense
classification, for instance).

## Sign convention

Internally, amounts keep their natural ledger sign: Assets and Expenses are debit-normal
(positive), Liabilities, Equity, and Revenue are credit-normal (negative) — this is exactly what
the trial balance report also relies on. For a balance sheet, though, a credit-normal balance is
supposed to read as a positive number (nobody wants to see their retained earnings printed as
`-5000.00`), so Liability and Equity rows are sign-flipped for display: `display = -raw`.
Asset rows are shown as-is.

## Net income: why it's folded into Equity

A *real* trial-balance-derived balance sheet only balances after period-end **closing entries**
move each Revenue and Expense account's balance into Equity (traditionally into a Retained
Earnings account), zeroing out the income statement accounts for the new period. ferro_ledger
v1 has no closing-entry mechanism — there's no `close` command, and nothing in
`docs/JOURNAL_FORMAT.md` models a fiscal period boundary.

Without closing entries, `Assets` and `Liabilities + Equity` alone will *not* balance whenever
there's been any revenue or expense activity that isn't purely balance-sheet-to-balance-sheet
(e.g. `Assets:Bank:Checking` funded by `Income:Sales`, which is exactly what
`examples/sample.journal` does). Rather than ship a report that's usually wrong by the amount of
unclosed net income, the balance sheet computes it itself and folds it into Equity as a single
synthetic row:

```
Net Income (unclosed) = -(sum of all Revenue account balances + sum of all Expense account balances)
```

(Revenue balances are already negative in the ledger's sign convention, Expense balances are
already positive; the formula above is just "revenue minus expenses," rearranged to work
directly on the raw ledger balances.) This row is not a real account — it doesn't exist in the
journal, has no `SourcePos`, and can't be posted to. It just makes the two sides of the
accounting equation match, exactly the way a real closing entry would, without requiring the
user to write one.

If your books *do* explicitly close income/expenses into an Equity account at period end (so
`Income:*`/`Expenses:*` are already zero at the point you run the report), the computed net
income will be zero and this row won't appear at all — `build()` only adds it when nonzero.

## Accounts the classifier doesn't recognize

An account whose top-level segment isn't one of the six recognized names (case-insensitive) is
**excluded from every section** — it's neither an asset, a liability, nor folded into equity.
This is deliberate: silently guessing wrong about an account's type would be worse than leaving
it out. Instead, `BalanceSheetReport::unclassified_accounts` collects every such account with a
nonzero balance, and the rendered report lists them in a trailing `NOTE:` block so a naming
mistake (e.g. an account under `Bank:` instead of `Assets:Bank:`) is visible rather than quietly
making the sheet balance for the wrong reason (or not balance, with no explanation).

## Multiple commodities

Same rule as the trial balance (`docs/JOURNAL_FORMAT.md`, "Deliberately unsupported in v1"): no
report converts across commodities. The balance sheet builds one independent sheet — with its
own Assets/Liabilities/Equity sections and its own accounting-equation check — per commodity
found in the ledger's Asset/Liability/Equity/Revenue/Expense balances.

## Relationship to the trial balance and clearing reports

- The **trial balance** (`docs/PLANNING.md` Phase 2) lists every account with a nonzero balance,
  full stop — it doesn't classify or exclude anything, which is why it stays the right report for
  "does this journal parse and balance at all."
- The **balance sheet** is a curated view over a subset of the same data (only
  Assets/Liabilities/Equity, plus net income folded in), and can legitimately fail to balance in
  a way the trial balance never will — e.g. if the required closing hasn't been reflected and net
  income folding still doesn't line up because of a genuine bug, or because of unclassified
  accounts pulled out of the equation. Check `unclassified_accounts` first when a balance sheet
  unexpectedly doesn't balance.
- **Clearing accounts** (`docs/CLEARING_ACCOUNTS.md`) are ordinary Asset (or Liability) accounts
  from the balance sheet's point of view — an outstanding clearing balance shows up as a normal
  Asset/Liability line, same as any other account. The balance sheet has no special awareness of
  clearing semantics; run `ferro_ledger clear` separately to see what's driving that balance.
