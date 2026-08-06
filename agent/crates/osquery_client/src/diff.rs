use crate::types::OsqueryRow;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

/// A wrapper around OsqueryRow to allow hashing and equality comparisons.
/// We sort the keys to ensure consistent hashing.
#[derive(Debug, Clone)]
struct HashableRow(OsqueryRow);

impl PartialEq for HashableRow {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for HashableRow {}

impl Hash for HashableRow {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut keys: Vec<&String> = self.0.keys().collect();
        keys.sort();
        for key in keys {
            key.hash(state);
            if let Some(val) = self.0.get(key) {
                val.hash(state);
            }
        }
    }
}

/// Computes the differential between two sets of rows.
/// Returns (added_rows, removed_rows).
pub fn compute_diff(
    previous_rows: &[OsqueryRow],
    current_rows: &[OsqueryRow],
) -> (Vec<OsqueryRow>, Vec<OsqueryRow>) {
    let mut prev_set: HashSet<HashableRow> = HashSet::new();
    for row in previous_rows {
        prev_set.insert(HashableRow(row.clone()));
    }

    let mut curr_set: HashSet<HashableRow> = HashSet::new();
    for row in current_rows {
        curr_set.insert(HashableRow(row.clone()));
    }

    let added: Vec<OsqueryRow> = curr_set
        .difference(&prev_set)
        .map(|h_row| h_row.0.clone())
        .collect();

    let removed: Vec<OsqueryRow> = prev_set
        .difference(&curr_set)
        .map(|h_row| h_row.0.clone())
        .collect();

    (added, removed)
}
