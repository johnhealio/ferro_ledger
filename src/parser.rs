//! Hand-written parser for the hledger-compatible journal subset documented in
//! `docs/JOURNAL_FORMAT.md`. Turns journal text into `Vec<Transaction>` (plus any `include`
//! directives found, which `journal.rs` is responsible for resolving and recursing into).

use crate::model::{Account, Amount, Posting, SourcePos, Status, Tag, Transaction};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::str::FromStr;

#[derive(Debug, thiserror::Error)]
#[error("{file}:{line}: {message}")]
pub struct ParseError {
    pub file: String,
    pub line: usize,
    pub message: String,
}

impl ParseError {
    fn new(file: &str, line: usize, message: impl Into<String>) -> Self {
        ParseError {
            file: file.to_string(),
            line,
            message: message.into(),
        }
    }
}

/// An `include PATH` directive found while parsing, with the path exactly as written (relative
/// resolution is `journal.rs`'s job, since it knows the including file's directory).
#[derive(Debug, Clone)]
pub struct IncludeDirective {
    pub path: String,
    pub source: SourcePos,
}

#[derive(Debug, Default)]
pub struct ParsedJournal {
    pub transactions: Vec<Transaction>,
    pub includes: Vec<IncludeDirective>,
}

/// Parses one journal file's text. `filename` is used only for error messages / `SourcePos`.
pub fn parse_str(content: &str, filename: &str) -> Result<ParsedJournal, ParseError> {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = ParsedJournal::default();

    let mut i = 0usize; // 0-based index into `lines`
    while i < lines.len() {
        let line_no = i + 1;
        let raw = lines[i];

        if raw.trim().is_empty() {
            i += 1;
            continue;
        }

        let is_indented = raw.starts_with(' ') || raw.starts_with('\t');
        let trimmed = raw.trim();

        if is_indented {
            // A stray indented line with no preceding transaction header is a parse error —
            // it can't belong to anything.
            return Err(ParseError::new(
                filename,
                line_no,
                "indented line with no preceding transaction header",
            ));
        }

        if trimmed.starts_with(';') || trimmed.starts_with('#') {
            i += 1;
            continue;
        }

        if let Some(rest) = strip_directive(trimmed, "include") {
            result.includes.push(IncludeDirective {
                path: rest.trim().to_string(),
                source: SourcePos {
                    file: filename.to_string(),
                    line: line_no,
                },
            });
            i += 1;
            continue;
        }

        if strip_directive(trimmed, "account").is_some()
            || strip_directive(trimmed, "commodity").is_some()
            || strip_directive(trimmed, "year").is_some()
            || strip_directive(trimmed, "Y").is_some()
        {
            i += 1;
            continue;
        }

        if let Some(first_token) = trimmed.split_whitespace().next()
            && try_parse_date_token(first_token).is_some() {
                let (txn, consumed) =
                    parse_transaction(&lines, i, filename)?;
                result.transactions.push(txn);
                i += consumed;
                continue;
            }

        return Err(ParseError::new(
            filename,
            line_no,
            format!("unrecognized directive or transaction line: '{}'", trimmed),
        ));
    }

    Ok(result)
}

fn strip_directive<'a>(line: &'a str, keyword: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(keyword)?;
    if rest.is_empty() {
        return Some("");
    }
    rest.starts_with(char::is_whitespace).then_some(rest)
}

fn try_parse_date_token(tok: &str) -> Option<NaiveDate> {
    let date_part = tok.split('=').next().unwrap_or(tok);
    for fmt in ["%Y-%m-%d", "%Y/%m/%d", "%Y.%m.%d"] {
        if let Ok(d) = NaiveDate::parse_from_str(date_part, fmt) {
            return Some(d);
        }
    }
    None
}

/// Parses one transaction starting at `lines[start]` (the header line). Returns the transaction
/// and the number of lines consumed (header + postings/comments).
fn parse_transaction(
    lines: &[&str],
    start: usize,
    filename: &str,
) -> Result<(Transaction, usize), ParseError> {
    let line_no = start + 1;
    let header = lines[start];
    let (main, comment) = split_inline_comment(header.trim());

    let mut rest = main;
    let first_token = rest
        .split_whitespace()
        .next()
        .ok_or_else(|| ParseError::new(filename, line_no, "empty transaction header"))?;
    let date_token = first_token;
    rest = rest[first_token.len()..].trim_start();

    let (date, secondary_date) = {
        let mut parts = date_token.splitn(2, '=');
        let d = parts.next().unwrap();
        let date = try_parse_date_token(d)
            .ok_or_else(|| ParseError::new(filename, line_no, format!("invalid date '{}'", d)))?;
        let secondary = match parts.next() {
            Some(s) => Some(try_parse_date_token(s).ok_or_else(|| {
                ParseError::new(filename, line_no, format!("invalid secondary date '{}'", s))
            })?),
            None => None,
        };
        (date, secondary)
    };

    let mut status = Status::Unmarked;
    if let Some(next_char) = rest.chars().next()
        && (next_char == '*' || next_char == '!') {
            let boundary_ok = rest
                .chars()
                .nth(1)
                .map(|c| c.is_whitespace())
                .unwrap_or(true);
            if boundary_ok {
                status = if next_char == '*' {
                    Status::Cleared
                } else {
                    Status::Pending
                };
                rest = rest[1..].trim_start();
            }
        }

    let mut code = None;
    if rest.starts_with('(') {
        if let Some(close) = rest.find(')') {
            code = Some(rest[1..close].to_string());
            rest = rest[close + 1..].trim_start();
        } else {
            return Err(ParseError::new(
                filename,
                line_no,
                "unterminated '(' code in transaction header",
            ));
        }
    }

    let description = rest.trim().to_string();
    let tags = comment.map(parse_tags).unwrap_or_default();

    let mut transaction = Transaction {
        date,
        secondary_date,
        status,
        code,
        description,
        comment: comment.map(|s| s.to_string()),
        tags,
        postings: Vec::new(),
        source: SourcePos {
            file: filename.to_string(),
            line: line_no,
        },
    };

    let mut i = start + 1;
    while i < lines.len() {
        let raw = lines[i];
        if raw.trim().is_empty() {
            break;
        }
        let is_indented = raw.starts_with(' ') || raw.starts_with('\t');
        if !is_indented {
            break;
        }
        let content = raw.trim_start();
        let cur_line_no = i + 1;

        if content.starts_with(';') || content.starts_with('#') {
            let comment_text = content.trim_start_matches([';', '#']).trim();
            for tag in parse_tags(comment_text) {
                transaction.tags.push(tag);
            }
            i += 1;
            continue;
        }

        let posting = parse_posting(content, filename, cur_line_no)?;
        transaction.postings.push(posting);
        i += 1;
    }

    balance_transaction(&mut transaction, filename)?;

    Ok((transaction, i - start))
}

fn parse_posting(content: &str, filename: &str, line_no: usize) -> Result<Posting, ParseError> {
    let (main, comment) = split_inline_comment(content.trim_end());
    let mut main = main.trim();

    let mut status = None;
    if let Some(next_char) = main.chars().next()
        && (next_char == '*' || next_char == '!') {
            let boundary_ok = main.chars().nth(1).map(|c| c.is_whitespace()).unwrap_or(true);
            if boundary_ok {
                status = Some(if next_char == '*' {
                    Status::Cleared
                } else {
                    Status::Pending
                });
                main = main[1..].trim_start();
            }
        }

    let (account_str, amount_str) = split_account_amount(main);
    if account_str.is_empty() {
        return Err(ParseError::new(filename, line_no, "posting has no account"));
    }

    let (amount, was_elided) = match amount_str {
        Some(a) if !a.trim().is_empty() => {
            let amt = parse_amount(&a)
                .map_err(|e| ParseError::new(filename, line_no, e))?;
            (Some(amt), false)
        }
        _ => (None, true),
    };

    let tags = comment.map(parse_tags).unwrap_or_default();

    Ok(Posting {
        account: Account::new(account_str),
        amount,
        was_elided,
        status,
        comment: comment.map(|s| s.to_string()),
        tags,
        source: SourcePos {
            file: filename.to_string(),
            line: line_no,
        },
    })
}

/// Splits a line into (content_before_comment, comment_text_without_leading_';') at the first
/// unquoted `;`. v1 has no quoting, so this is simply the first `;`.
fn split_inline_comment(line: &str) -> (&str, Option<&str>) {
    match line.find(';') {
        Some(idx) => (&line[..idx], Some(line[idx + 1..].trim())),
        None => (line, None),
    }
}

/// Splits posting content into (account, Some(amount_text)) or (account, None) if there's no
/// amount field (elided). The separator hledger uses is two-or-more spaces, or a tab.
fn split_account_amount(s: &str) -> (String, Option<String>) {
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\t' {
            let account: String = chars[..i].iter().collect::<String>().trim_end().to_string();
            let rest: String = chars[i + 1..].iter().collect::<String>().trim().to_string();
            return (account, if rest.is_empty() { None } else { Some(rest) });
        }
        if chars[i] == ' ' && chars.get(i + 1) == Some(&' ') {
            let account: String = chars[..i].iter().collect::<String>().trim_end().to_string();
            let mut j = i;
            while j < chars.len() && chars[j] == ' ' {
                j += 1;
            }
            let rest: String = chars[j..].iter().collect::<String>().trim().to_string();
            return (account, if rest.is_empty() { None } else { Some(rest) });
        }
        i += 1;
    }
    (s.trim().to_string(), None)
}

/// Parses a single amount field like `100.00 USD`, `$100.00`, `-$1,234.56`, `USD -100`.
fn parse_amount(s: &str) -> Result<Amount, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty amount".to_string());
    }
    let chars: Vec<char> = s.chars().collect();

    let first_digit = chars
        .iter()
        .position(|c| c.is_ascii_digit())
        .ok_or_else(|| format!("no numeric value found in amount '{}'", s))?;

    let prefix = &chars[..first_digit];
    let negative = prefix.contains(&'-');
    let prefix_symbol: String = prefix
        .iter()
        .filter(|c| **c != '-' && **c != '+')
        .collect::<String>()
        .trim()
        .to_string();

    let mut end = first_digit;
    while end < chars.len() && (chars[end].is_ascii_digit() || chars[end] == '.' || chars[end] == ',') {
        end += 1;
    }
    let numeral_raw = &chars[first_digit..end];
    let numeral = normalize_numeral(numeral_raw);

    let suffix: String = chars[end..].iter().collect::<String>().trim().to_string();

    let commodity = if !prefix_symbol.is_empty() {
        prefix_symbol
    } else {
        suffix
    };

    let numeral_signed = if negative {
        format!("-{}", numeral)
    } else {
        numeral
    };

    let quantity = Decimal::from_str(&numeral_signed)
        .map_err(|e| format!("invalid numeric amount '{}': {}", s, e))?;

    Ok(Amount::new(quantity, commodity))
}

/// Applies hledger's decimal-mark heuristic: the rightmost `.`/`,` present is the decimal
/// point; any other separator characters are thousands separators and are discarded. A numeral
/// with no separator at all is returned unchanged (an integer).
fn normalize_numeral(raw: &[char]) -> String {
    match raw.iter().rposition(|&c| c == '.' || c == ',') {
        None => raw.iter().collect(),
        Some(idx) => {
            let int_part: String = raw[..idx].iter().filter(|c| c.is_ascii_digit()).collect();
            let frac_part: String = raw[idx + 1..].iter().filter(|c| c.is_ascii_digit()).collect();
            let int_part = if int_part.is_empty() { "0".to_string() } else { int_part };
            if frac_part.is_empty() {
                int_part
            } else {
                format!("{}.{}", int_part, frac_part)
            }
        }
    }
}

/// Parses comma-separated `key:value` fragments out of a comment. A fragment with no `:` is
/// left as plain comment text (not a tag) — see `docs/JOURNAL_FORMAT.md`.
fn parse_tags(comment: &str) -> Vec<Tag> {
    comment
        .split(',')
        .filter_map(|fragment| {
            let fragment = fragment.trim();
            if fragment.is_empty() {
                return None;
            }
            let (key, value) = fragment.split_once(':')?;
            let key = key.trim().to_string();
            let value = value.trim();
            Some(Tag::new(key, if value.is_empty() { None } else { Some(value.to_string()) }))
        })
        .collect()
}

/// Validates that a transaction's postings sum to zero per commodity, inferring the amount of a
/// single elided posting if present. Mutates `transaction.postings` in place to fill in the
/// inferred amount.
fn balance_transaction(transaction: &mut Transaction, filename: &str) -> Result<(), ParseError> {
    let line_no = transaction.source.line;

    if transaction.postings.is_empty() {
        return Err(ParseError::new(
            filename,
            line_no,
            "transaction has no postings",
        ));
    }

    let elided_indices: Vec<usize> = transaction
        .postings
        .iter()
        .enumerate()
        .filter(|(_, p)| p.was_elided)
        .map(|(i, _)| i)
        .collect();

    if elided_indices.len() > 1 {
        return Err(ParseError::new(
            filename,
            line_no,
            "only one posting per transaction may elide its amount",
        ));
    }

    // Sum non-elided postings per commodity.
    let mut totals: Vec<(String, Decimal)> = Vec::new();
    for posting in transaction.postings.iter().filter(|p| !p.was_elided) {
        let amount = posting.amount.as_ref().expect("non-elided posting has amount");
        match totals.iter_mut().find(|(c, _)| c == &amount.commodity) {
            Some((_, total)) => *total += amount.quantity,
            None => totals.push((amount.commodity.clone(), amount.quantity)),
        }
    }

    if let Some(&elided_idx) = elided_indices.first() {
        let commodity = match totals.len() {
            0 => {
                return Err(ParseError::new(
                    filename,
                    line_no,
                    "cannot infer elided posting amount: no other postings to balance against",
                ));
            }
            1 => totals[0].0.clone(),
            _ => {
                return Err(ParseError::new(
                    filename,
                    line_no,
                    "cannot infer elided posting amount: postings span multiple commodities",
                ));
            }
        };
        let total = totals.iter().find(|(c, _)| *c == commodity).unwrap().1;
        transaction.postings[elided_idx].amount = Some(Amount::new(-total, commodity));
    } else {
        for (commodity, total) in &totals {
            if !total.is_zero() {
                return Err(ParseError::new(
                    filename,
                    line_no,
                    format!(
                        "transaction does not balance: {} '{}' postings sum to {} instead of 0",
                        transaction.description, commodity, total
                    ),
                ));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_balanced_transaction() {
        let src = "2024-01-15 * Payment received\n    Assets:Bank:Checking   100.00 USD\n    Income:Sales           -100.00 USD\n";
        let parsed = parse_str(src, "test.journal").expect("should parse");
        assert_eq!(parsed.transactions.len(), 1);
        let txn = &parsed.transactions[0];
        assert_eq!(txn.status, Status::Cleared);
        assert_eq!(txn.description, "Payment received");
        assert_eq!(txn.postings.len(), 2);
        assert_eq!(txn.postings[0].amount.as_ref().unwrap().quantity, Decimal::from_str("100.00").unwrap());
    }

    #[test]
    fn infers_elided_amount() {
        let src = "2024-01-16 Grocery shopping\n    Expenses:Groceries   45.00 USD\n    Assets:Bank:Checking\n";
        let parsed = parse_str(src, "test.journal").expect("should parse");
        let txn = &parsed.transactions[0];
        let elided = &txn.postings[1];
        assert!(elided.was_elided);
        assert_eq!(elided.amount.as_ref().unwrap().quantity, Decimal::from_str("-45.00").unwrap());
    }

    #[test]
    fn rejects_unbalanced_transaction() {
        let src = "2024-01-16 Bad entry\n    Expenses:Groceries   45.00 USD\n    Assets:Bank:Checking  -40.00 USD\n";
        let err = parse_str(src, "test.journal").unwrap_err();
        assert!(err.message.contains("does not balance"));
    }

    #[test]
    fn parses_tags_from_comment() {
        let src = "2024-03-01 * Customer payment\n    Assets:Clearing:Payments   500.00 USD  ; match:INV-2044\n    Income:Sales              -500.00 USD\n";
        let parsed = parse_str(src, "test.journal").expect("should parse");
        let posting = &parsed.transactions[0].postings[0];
        assert_eq!(posting.tag("match").unwrap().value.as_deref(), Some("INV-2044"));
    }

    #[test]
    fn parses_dollar_prefixed_amount() {
        let amt = parse_amount("-$1,234.56").unwrap();
        assert_eq!(amt.commodity, "$");
        assert_eq!(amt.quantity, Decimal::from_str("-1234.56").unwrap());
    }

    #[test]
    fn parses_include_directive() {
        let src = "include other.journal\n";
        let parsed = parse_str(src, "test.journal").expect("should parse");
        assert_eq!(parsed.includes.len(), 1);
        assert_eq!(parsed.includes[0].path, "other.journal");
    }

    #[test]
    fn rejects_unrecognized_directive() {
        let src = "~ monthly rent\n";
        let err = parse_str(src, "test.journal").unwrap_err();
        assert!(err.message.contains("unrecognized"));
    }
}
