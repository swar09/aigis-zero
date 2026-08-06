import asyncio
import sqlite3
import time
from pathlib import Path


class EventBuffer:
    """
    Local SQLite-backed buffer for JSON-encoded AgentEvent objects.

    Used when the Fleet server is unreachable.
    """

    def __init__(
        self,
        db_path: Path,
        max_events: int,
    ):
        self.db_path = Path(db_path)
        self.max_events = max_events

        # Ensure parent directory exists
        self.db_path.parent.mkdir(
            parents=True,
            exist_ok=True,
        )

        self._initialize_database()

    def _get_connection(self) -> sqlite3.Connection:
        """
        Open a SQLite connection.

        A new connection is opened for each blocking operation.
        This is safer when using asyncio.to_thread().
        """

        return sqlite3.connect(
            self.db_path
        )

    def _initialize_database(self):
        """
        Create the database and event_buffer table.
        """

        conn = self._get_connection()

        try:
            # Equivalent to:
            # PRAGMA journal_mode = WAL
            conn.execute(
                "PRAGMA journal_mode=WAL"
            )

            # Equivalent to:
            # PRAGMA synchronous = NORMAL
            conn.execute(
                "PRAGMA synchronous=NORMAL"
            )

            conn.execute(
                """
                CREATE TABLE IF NOT EXISTS event_buffer (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    payload JSON NOT NULL,
                    created_at INTEGER NOT NULL
                )
                """
            )

            conn.commit()

        finally:
            conn.close()

    async def push(
        self,
        event_json: str,
    ) -> None:
        """
        Store a JSON-encoded AgentEvent.

        If the buffer exceeds max_events,
        delete the oldest events.
        """

        await asyncio.to_thread(
            self._push_sync,
            event_json,
        )

    def _push_sync(
        self,
        event_json: str,
    ) -> None:

        conn = self._get_connection()

        try:

            now = int(time.time())

            conn.execute(
                """
                INSERT INTO event_buffer
                    (payload, created_at)
                VALUES (?, ?)
                """,
                (
                    event_json,
                    now,
                ),
            )

            count = conn.execute(
                """
                SELECT COUNT(*)
                FROM event_buffer
                """
            ).fetchone()[0]

            if count > self.max_events:

                to_delete = (
                    count - self.max_events
                )

                conn.execute(
                    """
                    DELETE FROM event_buffer
                    WHERE id IN (
                        SELECT id
                        FROM event_buffer
                        ORDER BY id ASC
                        LIMIT ?
                    )
                    """,
                    (to_delete,),
                )

            conn.commit()

        finally:
            conn.close()

    async def drain(
        self,
        batch_size: int,
    ) -> list[str]:
        """
        Read and remove the oldest batch of events.

        Returns raw JSON strings.
        """

        return await asyncio.to_thread(
            self._drain_sync,
            batch_size,
        )

    def _drain_sync(
        self,
        batch_size: int,
    ) -> list[str]:

        conn = self._get_connection()

        try:

            # Equivalent to the Rust transaction
            conn.execute("BEGIN")

            rows = conn.execute(
                """
                SELECT id, payload
                FROM event_buffer
                ORDER BY id ASC
                LIMIT ?
                """,
                (batch_size,),
            ).fetchall()

            if not rows:

                conn.commit()

                return []

            ids = [
                row[0]
                for row in rows
            ]

            events = [
                row[1]
                for row in rows
            ]

            placeholders = ",".join(
                "?"
                for _ in ids
            )

            conn.execute(
                f"""
                DELETE FROM event_buffer
                WHERE id IN ({placeholders})
                """,
                ids,
            )

            conn.commit()

            return events

        except Exception:

            conn.rollback()

            raise

        finally:

            conn.close()

    async def len(self) -> int:
        """
        Return the number of buffered events.
        """

        return await asyncio.to_thread(
            self._len_sync
        )

    def _len_sync(self) -> int:

        conn = self._get_connection()

        try:

            count = conn.execute(
                """
                SELECT COUNT(*)
                FROM event_buffer
                """
            ).fetchone()[0]

            return int(count)

        finally:

            conn.close()

    async def is_empty(self) -> bool:
        """
        Return True if the buffer contains no events.
        """

        return await self.len() == 0
