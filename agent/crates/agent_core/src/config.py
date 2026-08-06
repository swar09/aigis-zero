from dataclasses import dataclass
from pathlib import Path
from typing import Optional
from uuid import UUID
import tomllib


@dataclass
class AgentServerConfig:
    bind_addr: str
    port: int


@dataclass
class AgentConfigSection:
    """
    Agent identity and runtime settings.
    """

    # UUID assigned after first enrollment.
    # None means the agent has not been enrolled yet.
    node_id: Optional[UUID]

    name: str
    log_level: str
    log_format: str
    log_dir: Path
    data_dir: Path
    event_buffer_db: Path
    event_buffer_max: int
    event_drain_batch: int
    event_drain_interval_secs: int

    server: AgentServerConfig


@dataclass
class OsqueryConfig:
    """
    OSQuery daemon configuration.
    """

    socket_path: Path
    conf_path: Path
    flags_path: Path
    pid_file: Path
    log_path: Path
    connect_timeout_secs: int
    query_timeout_secs: int


@dataclass
class FleetConfig:
    host: str
    port: int
    endpoint: str
    enrollment_secret: str
    tls_ca_cert: Path
    tls_client_cert: Optional[Path]
    tls_client_key: Optional[Path]
    heartbeat_interval_secs: int
    reconnect_interval_secs: int
    max_reconnect_attempts: int


@dataclass
class IsolationConfig:
    enabled: bool
    fleet_ip: str
    fleet_port: int


@dataclass
class AgentConfig:
    """
    Root agent configuration.
    """

    agent: AgentConfigSection
    osquery: OsqueryConfig
    fleet: FleetConfig
    isolation: IsolationConfig


# for toml file loading
def load_config(path: Path) -> AgentConfig:
    """
    Load and parse the agent TOML configuration file.

    """

    # Open the TOML file in binary mode.
    #
    # tomllib.load() expects a binary file object.
    with path.open("rb") as file:
        data = tomllib.load(file)

    
    # agent server
    server_data = data["agent"]["server"]

    agent_server = AgentServerSection(
        bind_addr=server_data["bind_addr"],
        port=server_data["port"],
    )


    #agent
    agent_data = data["agent"]

    node_id = agent_data.get("node_id")

    if node_id is not None:
        node_id = UUID(node_id)

    agent = AgentSection(
        node_id=node_id,
        name=agent_data["name"],
        log_level=agent_data["log_level"],
        log_format=agent_data["log_format"],
        log_dir=Path(agent_data["log_dir"]),
        data_dir=Path(agent_data["data_dir"]),
        event_buffer_db=Path(agent_data["event_buffer_db"]),
        event_buffer_max=agent_data["event_buffer_max"],
        event_drain_batch=agent_data["event_drain_batch"],
        event_drain_interval_secs=agent_data[
            "event_drain_interval_secs"
        ],
        server=agent_server,
    )


    # osquery
  
    osquery_data = data["osquery"]

    osquery = OsquerySection(
        socket_path=Path(osquery_data["socket_path"]),
        conf_path=Path(osquery_data["conf_path"]),
        flags_path=Path(osquery_data["flags_path"]),
        pid_file=Path(osquery_data["pid_file"]),
        log_path=Path(osquery_data["log_path"]),
        connect_timeout_secs=osquery_data[
            "connect_timeout_secs"
        ],
        query_timeout_secs=osquery_data[
            "query_timeout_secs"
        ],
    )

    # fleet

    fleet_data = data["fleet"]

    tls_client_cert = fleet_data.get("tls_client_cert")

    if tls_client_cert is not None:
        tls_client_cert = Path(tls_client_cert)

    tls_client_key = fleet_data.get("tls_client_key")

    if tls_client_key is not None:
        tls_client_key = Path(tls_client_key)

    fleet = FleetSection(
        host=fleet_data["host"],
        port=fleet_data["port"],
        endpoint=fleet_data["endpoint"],
        enrollment_secret=fleet_data["enrollment_secret"],
        tls_ca_cert=Path(fleet_data["tls_ca_cert"]),
        tls_client_cert=tls_client_cert,
        tls_client_key=tls_client_key,
        heartbeat_interval_secs=fleet_data[
            "heartbeat_interval_secs"
        ],
        reconnect_interval_secs=fleet_data[
            "reconnect_interval_secs"
        ],
        max_reconnect_attempts=fleet_data[
            "max_reconnect_attempts"
        ],
    )

    # isolation

    isolation_data = data["isolation"]

    isolation = IsolationSection(
        enabled=isolation_data["enabled"],
        fleet_ip=isolation_data["fleet_ip"],
        fleet_port=isolation_data["fleet_port"],
    )

    # root agent config

    return AgentConfig(
        agent=agent,
        osquery=osquery,
        fleet=fleet,
        isolation=isolation,
    )
