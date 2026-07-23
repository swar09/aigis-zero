from typing import Any, AsyncIterator, Optional


class Connection:
    """
    For communicating with the Fleet server.
    
    """

    def __init__(self, endpoint: str):
        self.endpoint = endpoint
        self.channel: Optional[Any] = None
        self.client: Optional[Any] = None
        self.connected = False

    async def connect(self) -> None:
        """
        Establish a connection to the Fleet server.
        """
        self.connected = True

    async def close(self) -> None:
        """
        Close the connection.
        """
        self.channel = None
        self.client = None
        self.connected = False

    async def register_agent(self, request: Any) -> Any:
        """
        Send RegisterAgent RPC.

        Args:
            request: RegisterRequest

        Returns:
            RegisterResponse
        """
        if not self.connected:
            raise RuntimeError("Not connected to Fleet server.")

        raise NotImplementedError(
            "RegisterAgent RPC will be implemented once gRPC stubs are available."
        )

    async def send_heartbeat(self, request: Any, token: str) -> Any:
        """
        Send Heartbeat RPC.

        Args:
            request: HeartbeatRequest
            token: Bearer token

        Returns:
            HeartbeatResponse
        """
        if not self.connected:
            raise RuntimeError("Not connected to Fleet server.")

        raise NotImplementedError(
            "Heartbeat RPC will be implemented once gRPC stubs are available."
        )

    async def open_stream(
        self,
        events: AsyncIterator[Any],
        token: str,
    ) -> AsyncIterator[Any]:
        """
        Open bidirectional event stream.

        Args:
            events: Async iterator of AgentEvent
            token: Bearer token

        Yields:
            ServerCommand
        """
        if not self.connected:
            raise RuntimeError("Not connected to Fleet server.")

        raise NotImplementedError(
            "EventStream RPC will be implemented once gRPC stubs are available."
        )
