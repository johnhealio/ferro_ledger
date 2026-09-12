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

## Scoping to a fiscal period

A real income statement is scoped to a fiscal period ("Q1 2024," "the year ended..."). Use
`--since`/`--until` to scope this report (and every other report command) to a date window
before anything else runs — see `docs/DATE_RANGE.md` for the full semantics
(inclusive-start/exclusive-end, accepted formats):

```sh
cargo run -- income-statement examples/sample.journal --since 2024-01-01 --until 2024-02-01
```

Without either flag, this report covers the *entire* journal, same as it always has — that's
still the default, not a special case.

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
disagree, that's a bug in one of the two reports, not an expected discrepancy — including when
run with matching `--since`/`--until` values, since both commands filter transactions the same
way before building their respective report from them.
