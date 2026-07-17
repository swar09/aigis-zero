import json
from typing import Any

from isolation import IsolationManager
from osquery-client import OsqueryCollector

from dataclasses import dataclass

@dataclass
class CommandHandler:
    osquery: OsqueryCollector
  isolation: IsolationManager

    async def handle(self, msg: dict[str, Any]) -> dict[str, str]:
        """
        Handle a command received from the Fleet server.
        """

        command = msg.get("command")

        if command is None:
            raise ValueError("missing command")

        command_type = command.get("type")

        if command_type == "isolate":
            isolate = command.get("isolate")

            if isolate:
                await self.isolation.isolate()

                return {
                    "status": "isolated"
                }

            else:
                await self.isolation.de_isolate()

                return {
                    "status": "unisolated"
                }

        elif command_type == "config_update":
            return {
                "status": "config_updated"
            }

        elif command_type == "ack":
            return {
                "status": "acked"
            }

        else:
            raise ValueError(
                f"unknown command type: {command_type}"
            )
