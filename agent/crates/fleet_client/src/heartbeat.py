import asyncio
import logging

from fleet-client.types import HeartbeatRequest


logger = logging.getLogger(__name__)


class HeartbeatManager:

    @staticmethod
    async def start(
        connection,
        token: str,
        node_id: str,
        interval_secs: int,
    ) -> None:

        async def heartbeat_loop():

            while True:
                await asyncio.sleep(interval_secs)

                logger.debug(
                    "Sending heartbeat for node %s",
                    node_id,
                )

                request = HeartbeatRequest(
                    node_id=node_id,
                    status="healthy",
                    events_buffered=0,
                )

                try:
                    await connection.send_heartbeat(
                        request,
                        token,
                    )

                except Exception as e:
                    logger.warning(
                        "Failed to send heartbeat: %s",
                        e,
                    )

        asyncio.create_task(heartbeat_loop())
