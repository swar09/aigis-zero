from typing import Optional
import asyncio
import logging

from fleet_client.src.connection import Connection
from fleet_client.src.enrollment import AgentEnrollment
from fleet_client.src.heartbeat import HeartbeatManager
from fleet_client.src.stream import EventStreamManager

logger = logging.getLogger(__name__)


class FleetClient:
    """
    High-level Fleet client.

    This class coordinates:
    - Connection management
    - Agent enrollment
    - Heartbeats
    - Event streaming
    """

    def __init__(self, endpoint: str):
        self.endpoint = endpoint

        self.connection = Connection(endpoint)

        self.node_id: Optional[str] = None
        self.token: Optional[str] = None

        self.event_queue: asyncio.Queue = asyncio.Queue(maxsize=100)
        self.command_queue: Optional[asyncio.Queue] = None

    async def connect(self) -> None:
        """
        Connect to the Fleet server.
        """
        await self.connection.connect()

    async def enroll(self, request):
        """
        Register this agent with the Fleet server.
        """

        result = await AgentEnrollment.enroll(
            self.connection,
            request,
        )

        self.node_id = result.node_id
        self.token = result.token

        return result

    async def start_heartbeat(
        self,
        interval_secs: int = 30,
    ) -> None:

        if self.token is None:
            raise RuntimeError("Agent has not been enrolled.")

        if self.node_id is None:
            raise RuntimeError("Missing node_id.")

        await HeartbeatManager.start(
            self.connection,
            self.token,
            self.node_id,
            interval_secs,
        )

    async def start_event_stream(self) -> None:

        if self.token is None:
            raise RuntimeError("Agent has not been enrolled.")

        self.command_queue = await EventStreamManager.start(
            self.connection,
            self.token,
            self.event_queue,
        )

    async def send_event(self, event) -> None:
        """
        Queue an event for transmission.
        """
        await self.event_queue.put(event)

    async def receive_command(self):
        """
        Wait for the next server command.
        """
        if self.command_queue is None:
            raise RuntimeError("Event stream has not been started.")

        return await self.command_queue.get()

    async def close(self):
        """
        Close the Fleet connection.
        """
        await self.connection.close()
