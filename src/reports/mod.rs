//! Reports are pure functions of a [`crate::ledger::Ledger`] that return a data structure, kept
//! separate from rendering (each submodule's `render`/`render_section`) so report logic stays
//! unit-testable without capturing stdout.

pub mod clearing;
pub mod trial_balance;
