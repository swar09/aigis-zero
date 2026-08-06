from dataclasses import dataclass
from enum import Enum
from typing import Any, Optional


class EventType(Enum):
    """
    Type of event being sent from agent to Fleet server.
    """

    OSQUERY = 0
    PROCESS = 1
    FILE = 2
    NETWORK = 3


class AgentStatus(Enum):
    """
    Current operational status of the agent.
    """

    HEALTHY = 0
    DEGRADED = 1
    ISOLATED = 2

    def as_str(self) -> str:
        """
        Return the string representation used by the Fleet server.
        """

        return {
            AgentStatus.HEALTHY: "healthy",
            AgentStatus.DEGRADED: "degraded",
            AgentStatus.ISOLATED: "isolated",
        }[self]


class ConnectionState(Enum):
    """
    Connection state of the gRPC channel.
    """

    CONNECTED = "connected"
    RECONNECTING = "reconnecting"
    DISCONNECTED = "disconnected"


@dataclass
class RegisterRequest:
    """
    Sent by the agent to register with the Fleet server.
    """

    hostname: str
    os_version: str
    agent_version: str
    machine_id: str


@dataclass
class RegisterResponse:
    """
    Returned by the Fleet server after successful enrollment.
    """

    node_id: str
    token: str
    config: Optional["AgentConfigPayload"] = None


@dataclass
class EnrollmentResult:
    """
    Internal result returned after enrollment completes.
    """

    node_id: str
    token: str
    config: Optional["AgentConfigPayload"] = None


@dataclass
class AgentEvent:
    """
    Event sent from the agent to the Fleet server.
    """

    node_id: str
    event_type: int
    payload: Any
    timestamp_ns: int
    sequence_id: str


@dataclass
class ServerCommand:
    """
    A command sent from the Fleet server to the agent.
    """

    command: Optional["ServerCommandType"] = None


@dataclass
class IsolateCommand:
    """
    Command to isolate or de-isolate the node.
    """

    isolate: bool
    reason: str


@dataclass
class ConfigUpdateCommand:
    """
    Command to update the agent configuration.
    """

    config: Optional["AgentConfigPayload"] = None


@dataclass
class AckCommand:
    """
    Acknowledgment that the server received a specific event.
    """

    sequence_id: str


@dataclass
class ServerCommandType:
    """
    The actual command variant.

    Rust uses an enum with variants:

        Isolate(IsolateCommand)
        ConfigUpdate(ConfigUpdateCommand)
        Ack(AckCommand)

    In Python, this wrapper stores one of those command objects.
    """

    type: str
    value: Any


@dataclass
class AgentConfigPayload:
    """
    Configuration payload sent from Fleet server to agent.
    """

    osquery_schedule: list["OsquerySchedule"]
    heartbeat_interval_secs: int
    batch_size: int


@dataclass
class OsquerySchedule:
    """
    A single scheduled query definition.
    """

    name: str
    query: str
    interval_secs: int


@dataclass
class HeartbeatRequest:
    """
    Periodic heartbeat sent from agent to Fleet server.
    """

    node_id: str
    status: str
    events_buffered: int


@dataclass
class HeartbeatResponse:
    """
    Fleet server's response to a heartbeat.
    """

    ok: bool