# Date-range filtering

`--since <DATE>` and `--until <DATE>` scope a report to a window of transaction dates. They're
available on every subcommand that builds a report from a `Ledger`: `balance`/`trial-balance`,
`balance-sheet`/`bs`, `income-statement`/`is`, and `clear`.

```sh
cargo run -- income-statement examples/sample.journal --since 2024-01-01 --until 2024-02-01
```

## Semantics: inclusive start, exclusive end

A transaction is in range when `since <= date < until`. This mirrors hledger's `-b/--begin`
(inclusive) / `-e/--end` (exclusive) convention, under clearer names. Exclusive-end is what lets
you write non-overlapping, adjacent periods without off-by-one errors:

```sh
--since 2024-01-01 --until 2024-02-01   # all of January
--since 2024-02-01 --until 2024-03-01   # all of February — no gap, no overlap with January
```

Either flag can be omitted: `--since` alone means "from that date onward, no upper bound";
`--until` alone means "everything before that date, no lower bound." Omitting both (the
default) means every transaction is in scope, exactly as before this feature existed.

## Accepted date formats

Same three formats journal dates themselves use (`docs/JOURNAL_FORMAT.md`): `YYYY-MM-DD`,
`YYYY/MM/DD`, `YYYY.MM.DD`. Both the journal parser and the CLI flags call the same function,
`parser::parse_date_str`, so there is exactly one place that knows what a valid date string looks
like. An invalid `--since`/`--until` value fails immediately with a message naming the flag and
the accepted formats — before the journal is even loaded.

## Where filtering happens

`main.rs` filters the loaded `Vec<Transaction>` — via `DateRange::filter` in
`src/date_range.rs` — *before* constructing a `Ledger` from it. Every report module
(`trial_balance`, `balance_sheet`, `income_statement`, `clearing`) stays exactly as unaware of
date ranges as it was before this feature: they're still pure functions of whichever
transactions the `Ledger` happens to hold. This is deliberate — it means a report's correctness
and a date range's correctness can be tested completely independently (see
`tests/date_range_tests.rs` for the filtering logic in isolation, and
`tests/cli_date_range_tests.rs` for the flags wired through the actual binary).

## Interaction with `clear`

`clear`'s clearing-group matching operates only on postings within the given window, same as
every other report. A group whose two legs straddle the boundary (e.g. an invoice recorded
inside the window, settled just outside it) will show as outstanding within that window even
though it fully clears once you widen the range — this is the expected, and only sensible,
behavior: "outstanding as of this period" is exactly the question `--until` is asking.

## `check` is intentionally not scoped

`ferro_ledger check` doesn't accept `--since`/`--until`. It validates that every transaction in
the journal balances — a property of each transaction in isolation, not of any subset — so a
date window has nothing meaningful to restrict there. (Trying to pass the flags to `check`
fails with clap's own "unexpected argument" error, which `tests/cli_date_range_tests.rs` checks
for directly.)

## Not implemented: a `--period` shorthand

hledger also accepts a single `--period` expression (`-p 2024`, `-p "jan-mar 2024"`, etc.) that
expands to an equivalent begin/end pair. ferro_ledger v1 only has the two explicit flags — no
period-expression parser. This was floated as a possible extra in `docs/INCOME_STATEMENT.md`
before this feature existed; `--since`/`--until` covers the same use cases with slightly more
typing, and a `--period` shorthand remains a reasonable future addition if it turns out to be
worth the parsing complexity.
