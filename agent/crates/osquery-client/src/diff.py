from __future__ import annotations

from typing import Dict, List, Tuple, FrozenSet

from types import OsqueryRow


def _row_to_hashable(row: OsqueryRow) -> FrozenSet[Tuple[str, str]]:
    """
    Convert a dictionary into a hashable representation.

    """

    # Example:
    #     {
    #         "pid": "123",
    #         "name": "bash"
    #     }

    # becomes

    #     frozenset({
    #         ("pid", "123"),
    #         ("name", "bash")
    #     })

    # This allows rows to be inserted into a Python set.
    
    return frozenset(row.items())


def compute_diff(
    previous_rows: List[OsqueryRow],
    current_rows: List[OsqueryRow],
) -> Tuple[List[OsqueryRow], List[OsqueryRow]]:
    """
    Compute added and removed rows.

    Returns:
        (added_rows, removed_rows)
    """

    previous_set = {_row_to_hashable(row) for row in previous_rows}
    current_set = {_row_to_hashable(row) for row in current_rows}

    added = current_set - previous_set
    removed = previous_set - current_set

    added_rows = [dict(row) for row in added]
    removed_rows = [dict(row) for row in removed]

    return added_rows, removed_rows
