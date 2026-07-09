from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from typing import Dict, List

# Scheduled Query Definition

@dataclass
class ScheduledQuery:
    """
    A scheduled query stored in SQLite.
    """

    name: str
    query: str
    interval_secs: int
    snapshot: bool


# OSQuery Response Types


OsqueryRow = Dict[str, str]


@dataclass
class QueryStatus:
    """
    Status returned by osquery.
    """

    code: int
    message: str


@dataclass
class QueryResponse:
    """
    Response returned from OsqueryClient.query().
    """

    status: QueryStatus
    rows: List[OsqueryRow]


# ==========================================================
# Processed Result Types
# ==========================================================

@dataclass
class ColumnEntry:
    name: str
    value: str


@dataclass
class OsqueryResultRow:
    columns: List[ColumnEntry]


class ResultAction(Enum):
    SNAPSHOT = 0
    ADDED = 1
    REMOVED = 2

    def as_str(self) -> str:
        return self.name


@dataclass
class OsqueryResult:
    """
    Final processed result ready to send to the server.
    """

    query_name: str
    agent_uuid: str
    timestamp_ns: int
    rows: List[OsqueryResultRow]
    action: ResultAction
