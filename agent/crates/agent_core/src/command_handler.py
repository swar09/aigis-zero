from typing import Any

from fleet_client.src.types import ServerCommand
from isolation.isolation_manager import IsolationManager
from osquery_client.src import OsqueryCollector


class CommandHandler:
    def __init__(
        self,
        osquery: OsqueryCollector,
        isolation: IsolationManager,
    ):
        self.osquery = osquery
        self.isolation = isolation

    async def handle(self, msg: ServerCommand) -> dict[str, Any]:
        command = msg.command

        if command is None:
            raise ValueError("missing command")

        # Replace these isinstance() checks with your generated protobuf
        # message types if they differ.
        if hasattr(command, "isolate"):
            if command.isolate:
                await self.isolation.isolate()
                return {"status": "isolated"}

            await self.isolation.de_isolate()
            return {"status": "unisolated"}

        if command.__class__.__name__ == "ConfigUpdate":
            return {"status": "config_updated"}

        if command.__class__.__name__ == "Ack":
            return {"status": "acked"}

        raise ValueError(f"Unknown command type: {type(command).__name__}")