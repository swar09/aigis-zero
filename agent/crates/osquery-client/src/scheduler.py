import asyncio
import logging
import sqlite3
import time
from pathlib import Path
from typing import Dict, List

from osquery_client import OsqueryClient
from diff import compute_diff
from types import (
    ScheduledQuery,
    OsqueryRow,
    OsqueryResult,
    OsqueryResultRow,
    ColumnEntry,
    ResultAction,
)

logger=logging.getLogger(__name__)

class QueryScheduler:

    def __init__(self, db_path: str):
        
        self.db_path = Path(db_path)

        self.previous_results: Dict[
            str,
            List[OsqueryRow]
        ] = {}

# Load sql queries

def load_queries(self) -> List[ScheduledQuery]:

    connection = sqlite3.connect(self.db_path)

    cursor = connection.cursor()

    cursor.execute(
        """
        SELECT
            name,
            query,
            interval_secs,
            snapshot
        FROM scheduled_queries
        """
    )

    rows = cursor.fetchall()

    connection.close()

    queries = []

    for row in rows:

        queries.append(

            ScheduledQuery(
                name=row[0],
                query=row[1],
                interval_secs=row[2],
                snapshot=bool(row[3]),
            )

        )

    return queries

#Update or Insert queries
def upsert_queries(
    self,
    queries: List[ScheduledQuery],
):

    connection = sqlite3.connect(self.db_path)

    cursor = connection.cursor()

    for query in queries:

        cursor.execute(
            """
            INSERT INTO scheduled_queries
            (name, query, interval_secs, snapshot)

            VALUES (?, ?, ?, ?)

            ON CONFLICT(name)

            DO UPDATE SET

            query=excluded.query,

            interval_secs=excluded.interval_secs,

            snapshot=excluded.snapshot
            """,

            (
                query.name,
                query.query,
                query.interval_secs,
                int(query.snapshot),
            ),

        )

    connection.commit()

    connection.close()

async def run(
    self,
    result_queue: asyncio.Queue,
    socket_path: str,
    agent_uuid: str,
):
    """
    Start all scheduled queries.
    """

    logger.info("Starting scheduler")

    queries = self.load_queries()

    tasks = []

    for query in queries:

        tasks.append(

            asyncio.create_task(

                self.run_query(
                    query,
                    result_queue,
                    socket_path,
                    agent_uuid,
                )

            )

        )

    await asyncio.gather(*tasks)


async def run_query(
    self,
    scheduled_query: ScheduledQuery,
    result_queue: asyncio.Queue,
    socket_path: str,
    agent_uuid: str,
):

    logger.info(
        "Starting scheduled query %s",
        scheduled_query.name,
    )

    client = await OsqueryClient.connect(
        socket_path
    )

    while True:

        try:

            response = await client.query(
                scheduled_query.query
            )

            # Process response
            # Compute diff
            # Build result
            # Put into queue

        except Exception:

            logger.exception(
                "Query %s failed",
                scheduled_query.name,
            )

        await asyncio.sleep(
            scheduled_query.interval_secs
        )

  response = await client.query(
      scheduled_query.query
  )
  
  # Process response
  # Compute diff
  # Build result
  # Put into queue
  @dataclass
  class QueryResponse:
      status: QueryStatus
      rows: List[OsqueryRow]
    
  if response.status.code != 0:
  
      logger.error(
          "Query %s failed: %s",
          scheduled_query.name,
          response.status.message,
      )
  
  current_rows = response.rows
  
  previous_rows = self.previous_results.get(
      scheduled_query.name,
      [],
  )
  
  #snapshot or differential
  if scheduled_query.snapshot:
  
      action = ResultAction.SNAPSHOT
  
      rows_to_send = current_rows
  
  else:
  
      added, removed = compute_diff(
          previous_rows,
          current_rows,
      )
      continue
  
  
  #save current rows
  self.previous_results[
      scheduled_query.name
  ] = current_rows
  
