//! Loads a journal file (and any `include`d files) into a flat `Vec<Transaction>`. This is the
//! only place file I/O happens for reading books — `parser.rs` itself never touches the
//! filesystem, so it stays unit-testable on in-memory strings.

use crate::model::Transaction;
use crate::parser::{parse_str, ParseError};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum JournalError {
    #[error("failed to read '{path}': {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Parse(#[from] ParseError),
    #[error("circular include detected at '{path}'")]
    CircularInclude { path: String },
}

pub fn load_journal(path: impl AsRef<Path>) -> Result<Vec<Transaction>, JournalError> {
    let mut visiting = HashSet::new();
    let mut transactions = Vec::new();
    load_recursive(path.as_ref(), &mut visiting, &mut transactions)?;
    Ok(transactions)
}

fn load_recursive(
    path: &Path,
    visiting: &mut HashSet<PathBuf>,
    out: &mut Vec<Transaction>,
) -> Result<(), JournalError> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if !visiting.insert(canonical.clone()) {
        return Err(JournalError::CircularInclude {
            path: path.display().to_string(),
        });
    }

    let content = fs::read_to_string(path).map_err(|e| JournalError::Io {
        path: path.display().to_string(),
        source: e,
    })?;
    let filename = path.display().to_string();
    let parsed = parse_str(&content, &filename)?;

    out.extend(parsed.transactions);

    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    for include in parsed.includes {
        let include_path = base_dir.join(&include.path);
        load_recursive(&include_path, visiting, out)?;
    }

    visiting.remove(&canonical);
    Ok(())
}
