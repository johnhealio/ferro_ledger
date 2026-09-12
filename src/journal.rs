//! Loads a journal file (and any `include`d files) into a flat `Vec<Transaction>`. This is the
//! only place file I/O happens for reading books — `parser.rs` itself never touches the
//! filesystem, so it stays unit-testable on in-memory strings.

use crate::model::Transaction;
use crate::parser::{parse_str, ParseError};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Everything that can go wrong loading a journal: the file couldn't be read, its contents
/// didn't parse (see [`ParseError`]), or an `include` chain loops back on itself.
#[derive(Debug, thiserror::Error)]
pub enum JournalError {
    /// A journal file (the top-level one, or one reached via `include`) couldn't be read.
    #[error("failed to read '{path}': {source}")]
    Io {
        /// The path that failed to read.
        path: String,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// The journal text didn't match the grammar in `docs/JOURNAL_FORMAT.md`.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// An `include` directive forms a cycle (a file including itself, directly or through a
    /// chain of other includes).
    #[error("circular include detected at '{path}'")]
    CircularInclude {
        /// The path whose inclusion would re-enter a file already being loaded.
        path: String,
    },
}

/// Loads `path` and recursively resolves any `include` directives it (or its includes) contain,
/// returning every transaction found across the whole chain. This is the only entry point that
/// touches the filesystem — see the module docs for why that separation matters.
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
