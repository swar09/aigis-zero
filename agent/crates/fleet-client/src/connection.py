import asyncio
import logging
from enum import Enum
from typing import Optional

from .types import ConnectionState


logger = logging.getLogger(__name__)


class FleetConnection:
    """
    Manages the connection to the Fleet server.

    Equivalent to the Rust FleetConnection struct.
    """

    def __init__(
        self,
        endpoint: str,
    ):
        self.endpoint = endpoint

        # Rust:
        #
        # channel: Option<Channel>
        #
        # Python:
        self.channel: Optional[object] = None

        # Rust watch::Sender<ConnectionState>
        #
        # Python equivalent:
        # Keep the current state and notify registered listeners.
        self.state = ConnectionState.DISCONNECTED

        self._state_listeners: list[
            asyncio.Queue[ConnectionState]
        ] = []

    async def connect(self):
        """
        Connect to the Fleet server.

        Retries forever using exponential backoff.

        Backoff:

            1 second
            2 seconds
            4 seconds
            8 seconds
            ...
            maximum 60 seconds
        """

        backoff = 1
        max_backoff = 60

        while True:

            await self._set_state(
                ConnectionState.RECONNECTING
            )

            logger.info(
                "Connecting to fleet server at %s...",
                self.endpoint,
            )

            try:

                # The actual network connection will be implemented
                # according to the transport used by the Fleet client.
                channel = await self._create_connection()

                logger.info(
                    "Successfully connected to fleet server."
                )

                await self._set_state(
                    ConnectionState.CONNECTED
                )

                self.channel = channel

                return channel

            except Exception as error:

                logger.warning(
                    "Failed to connect to fleet server: %s. "
                    "Retrying in %s seconds",
                    error,
                    backoff,
                )

            await asyncio.sleep(backoff)

            backoff = min(
                backoff * 2,
                max_backoff,
            )

    async def _create_connection(self):
        """
        Create the actual connection to the Fleet server.

        This will be implemented once we know which Python
        networking library the other Fleet client modules use.
        """

        raise NotImplementedError

    async def _set_state(
        self,
        state: ConnectionState,
    ) -> None:

        self.state = state

        for listener in self._state_listeners:

            await listener.put(state)

    def subscribe(
        self,
    ) -> asyncio.Queue[ConnectionState]:
        """
        Subscribe to connection state changes.
        """

        queue: asyncio.Queue[
            ConnectionState
        ] = asyncio.Queue()

        self._state_listeners.append(queue)

        return queue
