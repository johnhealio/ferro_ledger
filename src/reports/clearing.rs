//! Clearing/suspense-account analysis. See `docs/CLEARING_ACCOUNTS.md` for the full design —
//! this module implements exactly that: group a clearing account's postings by matching key,
//! net each group, and report cleared vs. outstanding.

use crate::ledger::Ledger;
use crate::model::{Account, SourcePos};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::fmt::Write as _;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum GroupKey {
    /// Grouped by an explicit `match:` tag value.
    Tag(String),
    /// No `match:` tag present: grouped by (commodity, absolute quantity) instead. See
    /// "Matching key" in `docs/CLEARING_ACCOUNTS.md` for why this is a coarser fallback.
    Fallback(String, Decimal),
}

pub struct GroupMember {
    pub date: NaiveDate,
    pub description: String,
    pub quantity: Decimal,
    pub commodity: String,
    pub source: SourcePos,
}

pub struct ClearingGroup {
    pub label: String,
    pub members: Vec<GroupMember>,
    pub net: Decimal,
    pub commodity: String,
    pub mixed_commodity: bool,
    pub cleared: bool,
}

pub struct ClearingAccountReport {
    pub account: Account,
    pub groups: Vec<ClearingGroup>,
}

impl ClearingAccountReport {
    pub fn cleared_count(&self) -> usize {
        self.groups.iter().filter(|g| g.cleared).count()
    }

    pub fn outstanding_count(&self) -> usize {
        self.groups.iter().filter(|g| !g.cleared).count()
    }
}

/// The tag used to explicitly link postings that belong to the same clearing group.
const MATCH_TAG: &str = "match";

pub fn analyze(ledger: &Ledger, accounts: &[Account]) -> Vec<ClearingAccountReport> {
    accounts.iter().map(|account| analyze_account(ledger, account)).collect()
}

fn analyze_account(ledger: &Ledger, account: &Account) -> ClearingAccountReport {
    let mut groups: BTreeMap<GroupKey, Vec<GroupMember>> = BTreeMap::new();

    for pr in ledger.postings_in(account) {
        let amount = match &pr.posting.amount {
            Some(a) => a,
            None => continue, // parser guarantees every posting has an amount post-balancing
        };

        let key = match pr.transaction.effective_tag(pr.posting, MATCH_TAG) {
            Some(tag) => GroupKey::Tag(tag.value.clone().unwrap_or_default()),
            None => GroupKey::Fallback(amount.commodity.clone(), amount.quantity.abs()),
        };

        groups.entry(key).or_default().push(GroupMember {
            date: pr.transaction.date,
            description: pr.transaction.description.clone(),
            quantity: amount.quantity,
            commodity: amount.commodity.clone(),
            source: pr.posting.source.clone(),
        });
    }

    let mut report_groups: Vec<ClearingGroup> = groups
        .into_iter()
        .map(|(key, members)| build_group(key, members))
        .collect();

    report_groups.sort_by(|a, b| a.cleared.cmp(&b.cleared).then_with(|| a.label.cmp(&b.label)));

    ClearingAccountReport {
        account: account.clone(),
        groups: report_groups,
    }
}

fn build_group(key: GroupKey, members: Vec<GroupMember>) -> ClearingGroup {
    let mut commodities: Vec<&str> = members.iter().map(|m| m.commodity.as_str()).collect();
    commodities.sort();
    commodities.dedup();
    let mixed_commodity = commodities.len() > 1;

    let commodity = commodities.first().unwrap_or(&"").to_string();
    let net: Decimal = members.iter().map(|m| m.quantity).sum();
    let cleared = !mixed_commodity && net.is_zero();

    let label = match key {
        GroupKey::Tag(value) => {
            if value.is_empty() {
                format!("{}:(no value)", MATCH_TAG)
            } else {
                format!("{}:{}", MATCH_TAG, value)
            }
        }
        GroupKey::Fallback(commodity, abs_qty) => {
            format!("amount {:.2} {}", abs_qty, commodity)
        }
    };

    ClearingGroup {
        label,
        members,
        net,
        commodity,
        mixed_commodity,
        cleared,
    }
}

pub fn render(reports: &[ClearingAccountReport]) -> String {
    let mut out = String::new();
    for report in reports {
        writeln!(out, "Clearing account: {}", report.account).unwrap();
        writeln!(
            out,
            "  {} cleared group(s), {} outstanding group(s)",
            report.cleared_count(),
            report.outstanding_count()
        )
        .unwrap();

        if report.groups.is_empty() {
            writeln!(out, "  (no postings)").unwrap();
        }

        for group in &report.groups {
            let status = if group.mixed_commodity {
                "ANOMALY (mixed commodities)"
            } else if group.cleared {
                "CLEARED"
            } else {
                "OUTSTANDING"
            };
            writeln!(
                out,
                "  [{}] {}  net {:.2} {} ({} posting(s))",
                status,
                group.label,
                group.net,
                group.commodity,
                group.members.len()
            )
            .unwrap();
            for member in &group.members {
                writeln!(
                    out,
                    "      {}  {:<40}  {:>14.2} {}  ({})",
                    member.date, member.description, member.quantity, member.commodity, member.source
                )
                .unwrap();
            }
        }
        out.push('\n');
    }
    out
}
