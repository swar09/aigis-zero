from osquery_client import OsqueryClient
from scheduler import QueryScheduler
import asyncio
from asyncio import Queue
import logging #for errors

logger=logging.getLogger(__name__)


class OsqueryConfig:

    def __init__(self, socket_path, db_path):
        self.socket_path = socket_path
        self.db_path = db_path


class OsqueryCollector:

    def __init__(self, config):
        self.config = config

    async def start(self, agent_uuid:str):
        try:
            result_queue = Queue(maxsize=100)
            self.scheduler = QueryScheduler(self.config.db_path)
            
            asyncio.create_task(
                scheduler.run(
                result_queue,
                self.config.socket_path,
                agent_uuid,
                )
            )

            logger.info("Scheduler started successfully")
            return result_queue

        except Exception:
            logger.exception("Failed to start osquery scheduler")
            raise

    
    async def live_query(self, sql:str):
        
        try:
            client = await OsqueryClient.connect(
                self.config.socket_path
            )
    
            return await client.live_query(sql)

        except Exception:
            logger.exception("Live query failed")
            raise


    
    async def update_schedule(self, queries):
        
        try:
            scheduler = QueryScheduler(
                self.config.db_path
            )
    
            await scheduler.upsert_queries(queries)

        except Exception:
            logger.exception("Failed to update scheduler")
            raise
     
