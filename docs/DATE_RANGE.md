# Date-range filtering

`--since <DATE>`/`--until <DATE>`, or the `--period <EXPR>` shorthand for both at once, scope a
report to a window of transaction dates. They're available on every subcommand that builds a
report from a `Ledger`: `balance`/`trial-balance`, `balance-sheet`/`bs`, `income-statement`/`is`,
and `clear`.

```sh
cargo run -- income-statement examples/sample.journal --since 2024-01-01 --until 2024-02-01
cargo run -- income-statement examples/sample.journal --period 2024-01   # equivalent, shorter
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

## Period shorthand (`--period`)

`--period <EXPR>` expands to an equivalent `--since`/`--until` pair, implemented in
`src/period.rs`. It's a deliberately small subset of hledger's much richer `-p`/`--period`
expression language — enough for the common cases, documented exhaustively here rather than
left to guesswork.

**Grammar:** `["from"] TERM ["to" TERM]`, where `TERM` is one of:

| Term form | Denotes | Example |
|---|---|---|
| `YYYY` | that whole calendar year | `2024` → 2024-01-01 through end of 2024 |
| `YYYY-MM` (`/` or `.` also accepted) | that whole calendar month | `2024-01` → all of January 2024 |
| `YYYY-MM-DD` (any format journal dates accept) | that single day | `2024-03-20` → just that day |

A single term denotes its own range. A `TERM to TERM` range spans from the **start** of the left
term to the **end** of the right term — the two terms don't need matching granularity:

```sh
--period 2024                    # the whole year
--period 2024-01                 # just January
--period "2024-01 to 2024-03"    # Q1: start of January through end of March
--period "from 2024-01-15 to 2024-02-20"   # a specific 37-day window
--period "2024-01 to 2024"       # start of January through end of that year (mixed granularity)
```

The leading `from` is optional and case-insensitive, as is `to`. Quote any expression containing
a space, since it's more than one shell word.

`--period` cannot be combined with `--since`/`--until` — clap rejects that combination directly
(`conflicts_with` on the `since`/`until` args in `cli.rs`) with its own "cannot be used with"
error, before `main.rs` ever sees the arguments. An invalid period expression (a garbled term, a
`to` with nothing on one side, an out-of-range month) fails with `error: invalid --period '<input>': <reason>`.

**Not implemented**: month/quarter names (`"jan 2024"`, `"Q1 2024"`), relative dates (`"today"`,
`"last month"`), recurring/interval periods (`"weekly"`, `"every 2 months"`), and hledger's
bare-dash range syntax (`2024-01-2024-03`, ambiguous with a plain date's own dashes — this is
exactly why ferro_ledger requires the word `to` as a separator instead). `--period` is purely an
additional convenience layered on top of `--since`/`--until`; anything expressible with those
two flags is still available directly, unchanged by this feature.
