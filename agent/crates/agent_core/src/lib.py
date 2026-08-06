from dataclasses import dataclass,asdict
import asyncio
import json
from typing import Optional
import uuid
import logging

from command_handler import CommandHandler
from event_buffer import EventBuffer
from fleet_client.src.lib import FleetClient
from fleet_client.src.types import AgentEvent, EventType
from osquery_client import OsqueryCollector
from osquery_client.src.types import OsqueryResult


CMD_MAX_BACKOFF_ERRORS = 8
CMD_BACKOFF_BASE_MS = 50
CMD_BACKOFF_CEILING_MS = 12_800
CMD_POLL_INTERVAL_MS = 5

logger=logging.getLogger(__name__)


@dataclass
class AgentCore:
    shutdown: asyncio.Event
    osquery: OsqueryCollector
    buffer: EventBuffer
    command_handler: CommandHandler
    fleet_client: FleetClient

    async def run(self, agent_uuid: str):
        """
        Start all background tasks and wait until shutdown.
        """
        pass


def encode_osquery_result(result: OsqueryResult) -> Optional[str]:
    """
    Converts an OsqueryResult into an AgentEvent JSON string.
    Returns None if serialization fails.
    """
    try:
        payload = asdict(result)
    except Exception as e:
        logger.warning(
            "Failed to serialize OsqueryResult payload for query %s: %s",
            result.query_name,
            e,
        )
        return None

    event = AgentEvent(
        node_id=result.agent_uuid,
        event_type=EventType.OSQUERY,
        payload=payload,
        timestamp_ns=result.timestamp_ns,
        sequence_id=str(uuid.uuid4()),
    )

    try:
        return json.dumps(asdict(event))
    except Exception as e:
        logger.warning(
            "Failed to serialize AgentEvent for query %s: %s",
            result.query_name,
            e,
        )
        return None


async def _osquery_polling_loop(
    self,
    results_rx,
    buffer,
    shutdown,
    agent_uuid,
):
    logger.info(
        "OSQuery polling task started",
        extra={"agent_uuid": agent_uuid},
    )

    while True:

        if shutdown.is_set():
            logger.info(
                "OSQuery polling task: shutdown signal received, draining remaining events"
            )

            while not results_rx.empty():

                result = await results_rx.get()

                event_json = encode_osquery_result(result)

                if event_json is not None:
                    try:
                        await buffer.push(event_json)
                    except Exception as e:
                        logger.error(
                            "Failed to buffer OSQuery result during shutdown drain: %s",
                            e,
                        )

            break

        result = await results_rx.get()

        event_json = encode_osquery_result(result)

        if event_json is None:
            continue

        logger.debug(
            "Buffering OSQuery event",
            extra={
                "query": result.query_name,
                "rows": len(result.rows),
            },
        )

        try:
            await buffer.push(event_json)

        except Exception as e:
            logger.error(
                "Failed to push OSQuery event to buffer; event dropped: %s",
                e,
            )

    logger.info("OSQuery polling task exited cleanly")


async def _command_listener_loop(self, shutdown):
    """
    Listen for commands from the Fleet server and dispatch them
    to the CommandHandler.
    """

    logger.info("Command listener task started")

    consecutive_errors = 0

    while True:

        # Shutdown check
        if shutdown.is_set():
            logger.info(
                "Command listener task: shutdown signal received, exiting"
            )
            break

        try:
            poll_result = await self.fleet_client.receive_command()
            
            consecutive_errors = 0

            logger.debug(
                "Received ServerCommand: %s",
                poll_result,
            )

            try:
                response = await self.command_handler.handle(
                    poll_result
                )

                logger.debug(
                    "Command handled successfully: %s",
                    response,
                )

            except Exception as e:

                logger.warning(
                    "CommandHandler returned error: %s",
                    e,
                )

        # --------------------------------
        # Transport error
        # --------------------------------
        except Exception as e:

            consecutive_errors += 1

            backoff_ms = min(
                CMD_BACKOFF_BASE_MS
                * (
                    2
                    ** min(
                        consecutive_errors,
                        CMD_MAX_BACKOFF_ERRORS,
                    )
                ),
                CMD_BACKOFF_CEILING_MS,
            )

            logger.error(
                "Command listener transport error: %s "
                "(attempt=%d, backoff=%d ms)",
                e,
                consecutive_errors,
                backoff_ms,
            )

            # Attempt reconnect
            try:

                await self.fleet_client.connect()
                
                logger.info(
                    "Successfully re-established connection"
                )

                consecutive_errors = 0

            except Exception as reconnect_error:

                logger.warning(
                    "Failed to reconnect: %s",
                    reconnect_error,
                )

    logger.info("Command listener task exited cleanly")


async def run(self, agent_uuid):

    shutdown = self.shutdown

    results_rx = await self.osquery.start(agent_uuid)

    buffer_task = self.buffer
    agent_uuid_owned = agent_uuid
    shutdown_osq = shutdown
    

    osquery_task = asyncio.create_task(
            self._osquery_polling_loop(
                results_rx,
                buffer_task,
                shutdown_osq,
                agent_uuid_owned,
            )
        )

    command_task = asyncio.create_task(
        self._command_listener_loop(
            shutdown,
        )
    )

    logger.info(
        "AgentCore: waiting for shutdown signal"
    )

    await shutdown.wait()

    logger.info(
        "AgentCore: shutdown signal received"
    )

    # wait for osquery task

    # wait for command task

    logger.info(
        "AgentCore: all tasks exited"
    )