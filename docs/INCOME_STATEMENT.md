# Income statement report

`ferro_ledger income-statement <journal>` (alias `is`) prints Revenue, Expenses, and the
resulting net income (or net loss) — the classic profit & loss statement.

## Classifying accounts

Same heuristic as the balance sheet (`docs/BALANCE_SHEET.md`, `src/account_types.rs`): an
account's top-level segment, case-insensitively, singular or plural, classifies it as
`Revenue` (`income`, `revenue`, `revenues`) or `Expense` (`expense`, `expenses`). Asset,
Liability, and Equity accounts are simply **out of scope** for this report — not shown, and not
treated as an anomaly, since a balance sheet account genuinely has nothing to do with a P&L. Only
an account whose top-level segment doesn't match *any* recognized type at all (not
Asset/Liability/Equity either) is flagged, in `unclassified_accounts`, for the same reason the
balance sheet flags them: a naming typo should be visible, not silently swallowed.

## Sign convention

Revenue accounts are credit-normal (negative in the ledger's raw sign convention); this report
flips them to display positive, same as the balance sheet does for Liabilities and Equity.
Expense accounts are already debit-normal (positive) and are shown as-is.

```
Net income = Total Revenue − Total Expenses
```

Rendered as `NET INCOME: <amount>` when nonnegative, `NET LOSS: <amount>` (still with its actual
sign) when negative.

## No date-range filtering (v1 limitation)

A real income statement is scoped to a fiscal period ("Q1 2024," "the year ended..."). v1 has no
date-range flags anywhere in the CLI — every report, including this one, covers the *entire*
journal every time, same limitation the trial balance and balance sheet already have. This means
`ferro_ledger income-statement` today answers "what's the cumulative net income across
everything in this journal file," not "what happened last month." A natural follow-up (not yet
implemented) would add `--since`/`--until` (or a single `--period`) flags shared across all
three reports, filtering transactions by `date` before anything else runs.

## Relationship to the balance sheet

This report and the balance sheet's folded-in "Net Income (unclosed)" Equity row
(`docs/BALANCE_SHEET.md`) are computed from the exact same Revenue/Expense balances, just
presented differently — the balance sheet needs a single number to satisfy the accounting
equation, this report shows how that number breaks down by account. They will always agree:

```
income_statement::build(&ledger).statements[i].net_income()
    == the corresponding balance_sheet::build(&ledger) sheet's "Net Income (unclosed)" row
```

`tests/income_statement_tests.rs` asserts this directly. If you ever see these two numbers
disagree, that's a bug in one of the two reports, not an expected discrepancy.
