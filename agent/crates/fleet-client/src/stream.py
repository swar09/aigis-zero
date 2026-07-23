import asyncio
import logging

from fleet-client.types import AgentEvent, ServerCommand

logger = logging.getLogger(__name__)


class EventStreamManager:

    @staticmethod
    async def start(
        connection,
        token: str,
        events_queue: asyncio.Queue,
    ) -> asyncio.Queue:

        command_queue = asyncio.Queue(maxsize=100)

        async def event_generator():
            while True:
                event = await events_queue.get()
                yield event

        async def receive_commands():
            try:
                async for command in connection.open_stream(
                    event_generator(),
                    token,
                ):
                    await command_queue.put(command)

            except Exception as e:
                logger.warning(
                    "Event stream closed: %s",
                    e,
                )

        asyncio.create_task(receive_commands())

        return command_queue
