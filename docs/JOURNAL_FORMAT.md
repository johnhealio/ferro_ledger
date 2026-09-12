# Journal format

ferro_ledger reads a **subset of hledger's journal format**. Anything accepted here should mean
the same thing to `hledger -f file.journal print` as it does to `ferro_ledger`. This document
describes exactly what the parser accepts; treat additions to the parser and additions to this
doc as one change.

Reference: https://hledger.org/hledger.html#journal-format (that page is the authority on hledger
itself; this doc only claims to cover the subset listed below).

## Transactions

```
2024-01-15 * (CHK-101) Payment received from customer
    ; whole-transaction comment line
    Assets:Bank:Checking          100.00 USD
    Income:Sales                 -100.00 USD  ; per-posting comment, tag:value

2024-01-16 Grocery shopping
    Expenses:Groceries             45.00 USD
    Assets:Bank:Checking                        ; elided — inferred as -45.00 USD
```

A transaction is:

```
DATE [=SECONDARY_DATE] [STATUS] [(CODE)] DESCRIPTION
    [indented comment line(s)]
    POSTING
    POSTING
    ...
```

- **DATE**: `YYYY-MM-DD`, `YYYY/MM/DD`, or `YYYY.MM.DD` (hledger accepts all three separators;
  parser normalizes to `NaiveDate`).
- **`=SECONDARY_DATE`** (optional): a second date after `=`, same format, for a posting/settlement
  date distinct from the primary transaction date.
- **STATUS** (optional): `*` (cleared) or `!` (pending). Absent means unmarked.
- **`(CODE)`** (optional): parenthesized reference code (check number, invoice id, etc).
- **DESCRIPTION**: free text to end of line. hledger further splits this into
  payee/note on a `|`; ferro_ledger keeps the whole thing as `description` in v1 (no payee/note
  split) — a documented subset limitation, not a divergence in what's *accepted*.
- Transactions are separated by one or more blank lines (or EOF/next transaction header).

## Postings

```
    ACCOUNT   AMOUNT [COMMODITY]  [; COMMENT]
    ACCOUNT                       [; COMMENT]      (elided amount)
```

- Indentation (at least one space/tab) marks a line as a posting or comment belonging to the
  current transaction; a non-indented line starts a new transaction/directive.
- **ACCOUNT**: colon-separated hierarchy, e.g. `Assets:Bank:Checking`. No declaration required to
  use an account (matches hledger's default — `account` directives are optional documentation,
  not registration, unless strict mode is on, which ferro_ledger v1 does not implement).
- **AMOUNT**: an optional sign, digits, optional decimal point, optional thousands separators
  (`,` or `.` — parser infers which is the decimal separator from position, same heuristic
  hledger uses: the rightmost `.`/`,` followed by exactly the fractional digits is the decimal
  point). Stored as `rust_decimal::Decimal` — never a float.
- **COMMODITY**: a symbol before or after the amount (`$100.00`, `100.00 USD`, `USD 100.00`).
  Free-standing letters/symbol adjacent to the number; whitespace between number and a
  following commodity code, no whitespace for a prefixed symbol — matches hledger's lexing.
- **Elided amount**: at most one posting per transaction may omit its amount. The parser infers
  it as the negation of the sum of the other postings' amounts (per commodity). Zero or more than
  one elided posting with an unbalanceable remainder is a parse error naming the transaction's
  source line.
- **Per-posting status**: a `*`/`!` may also prefix an individual posting's account, per hledger.

## Comments and tags

- A line whose first non-whitespace character is `;` or `#` is a full-line comment (top-level or
  indented under a transaction — either is a comment, not a posting).
- Inline comments start at an unquoted `;` anywhere after an account/amount on a posting line, or
  after the description on a transaction header line.
- **Tags**: inside a comment, `key:value` pairs separated by `,` are parsed as tags, e.g.
  `; match:INV-1042, note:partial payment` yields tags `match=INV-1042` and `note=partial payment`.
  A bare `key:` (colon, no value) is accepted as a tag with an empty value. A comma-separated
  fragment with **no** colon at all is left as plain comment text, not a tag — matching hledger
  (a tag always requires the colon). Tags on a transaction's own comment line apply to the
  transaction; tags on a posting's comment line apply to that posting only.

## Directives

Recognized and parsed; only `include` affects program behavior in v1, the rest are accepted so
that real hledger journals don't fail to parse, but are otherwise no-ops:

- `account NAME` — declares an account (no-op beyond acceptance; no strict-mode enforcement).
- `commodity SYMBOL` / `commodity FORMAT` — accepted, ignored.
- `include PATH` — recursively parses `PATH` (relative to the including file) and splices its
  transactions in. Cycles are a parse error.
- `; ` / `#` at column 0 — top-level comment line, ignored.
- `year YYYY` / `Y YYYY` — accepted, ignored in v1 (no partial-date `MM-DD` transactions
  supported yet, so there's nothing for the default year to apply to).

Anything else at column 0 that isn't a transaction date is a parse error naming the offending
line — silently skipping unrecognized directives would risk mis-parsing a real hledger file that
uses a feature we haven't implemented, which would violate the compatibility goal in `CLAUDE.md`.

## Balancing rule

Every transaction's postings must sum to exactly zero **per commodity**, after elided-amount
inference. This is enforced at parse time (`parser.rs`), not deferred to report time — a journal
that doesn't balance fails to load at all, matching `hledger check` / default `hledger` behavior
(unbalanced transactions are always an error, never a warning).

## Deliberately unsupported in v1 (subset limitations, not incompatibilities)

These are valid hledger syntax that ferro_ledger's parser does not yet accept; a file using them
will fail to parse here (with a clear error) even though `hledger` accepts it. Listed so nobody
mistakes a `NotImplemented` parse error for a compatibility bug:

- Payee/note split (`|` in description), auto-postings (`= query` rules), periodic transaction
  rules (`~`), budget goals, valuation/cost notation (`@`, `@@`), price directives (`P`),
  multi-line/`Y`-relative partial dates, `apply account`/`end apply account` blocks, `alias`
  directives, balance assertions (`=`/`==` after an amount).
- Multiple commodities is fine to *parse and store separately*, but no report in v1 converts or
  sums across commodities — each commodity gets its own column/section.
