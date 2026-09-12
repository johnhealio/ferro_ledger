# Clearing / suspense accounts

## The problem

Some balance-sheet accounts aren't meant to hold a balance long-term — they're a temporary
parking spot used to link two halves of a process that don't happen atomically. Classic
examples:

- **Bank clearing / suspense**: a customer payment hits `Assets:Clearing:Payments` when recorded,
  and a later bank-feed import moves the same amount from `Assets:Clearing:Payments` to
  `Assets:Bank:Checking` once it actually settles. Until the settlement transaction shows up,
  the clearing account carries an open balance for that one payment.
- **Inter-company / inter-fund suspense**: money leaves one book via a suspense account and is
  expected to be claimed by an offsetting entry (possibly in a different set of accounts) that
  nets it to zero.
- **Payroll clearing**: gross pay is debited out through a clearing account and the net-pay,
  tax, and benefits postings that follow are expected to net it back to zero.

In every case the account's balance *should* be zero once every transaction that was supposed to
land there has arrived — a nonzero balance means something is still outstanding (or something is
actually wrong). This document defines how ferro_ledger identifies which postings in a clearing
account belong together as a "group", and when a group counts as cleared.

## Matching key

Each posting in a designated clearing account is assigned a **matching key**:

1. If the posting (or its transaction) carries a `match:` tag, the key is `match:<value>`.
   This is the expected, explicit way to use a clearing account: give every leg of the same
   real-world event the same `match:` value.

   ```
   2024-03-01 * Customer payment recorded
       Assets:Clearing:Payments      500.00 USD   ; match:INV-2044
       Income:Sales                 -500.00 USD

   2024-03-04 * Bank settlement
       Assets:Bank:Checking          500.00 USD
       Assets:Clearing:Payments     -500.00 USD   ; match:INV-2044
   ```

2. If no `match:` tag is present, the key falls back to
   `(account, commodity, abs(quantity))` — i.e. postings of the same absolute amount and
   commodity in the same clearing account are assumed to be a pair. This fallback exists so a
   small/manual journal doesn't require tagging every single entry, but it is intentionally
   coarse: two *unrelated* postings of the same size in the same period will be (mis)matched
   together. **Recommendation, stated in the CLI help text and here: tag anything you care about
   reconciling correctly.**

Matching keys are scoped per clearing account — a key never matches across two different
clearing accounts, even if both were given to the `clear` command in the same invocation.

## Grouping and clearing rule

For a given clearing account and matching key, collect every posting in that account carrying
that key, across the whole journal (no date-window restriction in v1 — see "Future work"). A
group is:

- **Cleared** if its postings' quantities sum to exactly zero (per commodity — a group must be
  single-commodity to be evaluatable; a mixed-commodity group is reported as an error/anomaly,
  not silently ignored).
- **Outstanding** otherwise — the report shows the net remaining balance and lists the member
  transactions, so the user can see what's still open (e.g. a payment recorded but not yet
  settled, waiting on its other leg).

A group of exactly one posting is always outstanding (nothing to net against yet) unless its
amount is itself zero.

## What the `clear` report does (and doesn't do)

`ferro_ledger clear <journal> --account Assets:Clearing:Payments [--account ...]`:

- Computes the groups as above for each named account.
- Prints, per account: total cleared groups (count + total value that passed through and
  cancelled), and each outstanding group with its net balance and the transactions/postings
  contributing to it.
- **Does not modify the journal file.** v1's `clear` is read-only analysis, deliberately — see
  "Future work: mark-as-cleared" below for why writing back is left out of v1.

This makes `clear` safe to run at any time, including in a CI-style check, without risk of
mutating the source-of-truth text file.

## Relationship to hledger's transaction `*`/`!` status

hledger has a *transaction-level* status flag (`*` cleared / `!` pending / unmarked), typically
used for bank-reconciliation ("this transaction, as a whole, has been confirmed against my bank
statement"). ferro_ledger's `model::Status` implements that same flag for hledger-syntax
compatibility (see `docs/JOURNAL_FORMAT.md`) — but it is **not** what the `clear` report uses.

The clearing-account matching described in this document is a different, ledger-level concept:
whether a *group of postings inside one designated account* nets to zero. A transaction can be
marked `*` (confirmed against a bank feed) while its clearing-account posting is still part of an
outstanding group (e.g. confirmed as received, but the offsetting settlement hasn't posted yet),
and vice versa. Don't conflate the two when reading report output.

## Future work (explicitly not in v1)

- **`clear --mark`**: rewrite the journal, adding a `cleared:<group-id>` tag (or flipping the
  transaction's status to `*`) to every posting in a group once it nets to zero, so a rerun can
  skip re-deriving history. Left out of v1 because (a) it means the CLI mutates the user's source
  file, which needs its own care around idempotency/formatting-preservation, and (b) v1's
  recompute-every-run model is simpler and sufficient at expected journal sizes. Revisit if
  journals get large enough that re-grouping every run is slow, or if users want a persistent
  audit trail of *when* a group cleared.
- **Date-windowed matching** (only consider postings within N days of each other before treating
  them as a group) to reduce fallback false-positives without requiring tags everywhere.
- **Partial clearing**: today a group is binary (cleared/outstanding); a more advanced model
  could show partial settlement (e.g. 300 of 500 has cleared) for keys with more than two
  postings.
