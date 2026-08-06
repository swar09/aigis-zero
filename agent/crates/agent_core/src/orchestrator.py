import asyncio
import json
import logging
import os
import platform
import signal
import socket
import uuid
from pathlib import Path

import tomllib

from config import AgentConfig
from event_buffer import EventBuffer
from fleet_client.lib import FleetClient
from fleet_client.types import AgentEvent, EventType, RegisterRequest
from osquery_client import OsqueryCollector, OsqueryConfig
from osquery_client.types import OsqueryResult

import agent_tracing

logger = logging.getLogger(__name__)
AGENT_VERSION = "0.1.0"


# ------------------------------------------------------------
# Main entry point
# ------------------------------------------------------------

async def run():
    """
    Main agent orchestrator.
    """

    # --------------------------------------------------------
    # Load configuration
    # --------------------------------------------------------

    config_path = os.getenv("EDR_AGENT_CONFIG", "agent.toml")

    try:
        with open(config_path, "rb") as f:
            config_dict = tomllib.load(f)
    except Exception as e:
        raise RuntimeError(f"Failed to read config file {config_path}: {e}")

    config = AgentConfig.from_dict(config_dict)

    # --------------------------------------------------------
    # Initialize logging
    # --------------------------------------------------------

    log_format = (
        agent_tracing.LogFormat.JSON
        if config.agent.log_format.lower() == "json"
        else agent_tracing.LogFormat.HUMAN
    )

    agent_tracing.init(
        config.agent.log_level,
        log_format,
    )

    logger.info("Starting Aigis-Zero Agent Orchestrator")

    # --------------------------------------------------------
    # Watch config file
    # --------------------------------------------------------

    asyncio.create_task(watch_config(config_path))

    # --------------------------------------------------------
    # Event Buffer
    # --------------------------------------------------------

    buffer = EventBuffer(
        db_path=config.agent.event_buffer_db,
        max_events=config.agent.event_buffer_max,
    )

    logger.info(
        "Initialized event buffer at %s",
        config.agent.event_buffer_db,
    )

    # --------------------------------------------------------
    # Osquery Collector
    # --------------------------------------------------------

    collector = await OsqueryCollector.create(
        OsqueryConfig(
            socket_path=config.osquery.socket_path,
            db_path=config.agent.event_buffer_db,
        )
    )

    agent_uuid = (
        str(config.agent.node_id)
        if config.agent.node_id
        else "unregistered"
    )

    results_queue = await collector.start(agent_uuid)

    logger.info(
        "OsqueryCollector started (agent_uuid=%s)",
        agent_uuid,
    )

    # --------------------------------------------------------
    # Fleet Enrollment
    # --------------------------------------------------------

    logger.info(
        "Attempting fleet enrollment "
        "(non-fatal if server is unavailable)"
    )

    fleet = FleetClient(config.fleet.endpoint)

    request = RegisterRequest(
        hostname=hostname_or_default(),
        os_version=get_os_version(),
        agent_version=AGENT_VERSION,
        machine_id=read_machine_id(),
    )

    try:
        enrollment = await asyncio.wait_for(
            fleet.enroll(request),
            timeout=5,
        )

        logger.info("Enrolled with Fleet server. node_id=%s",
                    enrollment.node_id,
        )

    except asyncio.TimeoutError:
        logger.warning(
            "Fleet enrollment timed out. Running offline."
        )

    except Exception as e:
        logger.warning(
            "Fleet enrollment failed: %s",
            e,
        )

    # --------------------------------------------------------
    # Main Event Loop
    # --------------------------------------------------------

    logger.info("Agent is running")

    shutdown = asyncio.Event()

    shutdown_task = asyncio.create_task(
        wait_for_shutdown(shutdown)
    )
    
    while not shutdown.is_set():

        try:

            result = await asyncio.wait_for(
                results_queue.get(),
                timeout=1,
            )

        except asyncio.TimeoutError:
            continue

        encoded = encode_result(result)

        try:

            await buffer.push(encoded)

            logger.debug(
                "Buffered '%s' (%d rows, action=%s)",
                result.query_name,
                len(result.rows),
                result.action,
            )
        except Exception as e:

            logger.exception(
                "Failed to buffer result: %s",
                e,
            )

    logger.info("Agent shutting down")
    shutdown_task.cancel()


# ------------------------------------------------------------
# Config watcher
# ------------------------------------------------------------

async def watch_config(config_path: str):

    """
    Temporary polling implementation.

    Rust uses notify.
    Python can later be upgraded to watchdog/watchfiles.
    """

    logger.info("Watching %s for changes", config_path)

    last_modified = None

    while True:

        try:

            current = os.path.getmtime(config_path)

            if last_modified is None:

                last_modified = current

            elif current != last_modified:

                last_modified = current

                logger.info(
                    "Configuration modified "
                    "(hot reload TODO)"
                )

        except Exception:

            pass

        await asyncio.sleep(2)

# ----------------------------------------------------------------
# Shutdown
# ----------------------------------------------------------------

async def wait_for_shutdown(event: asyncio.Event):
    """
    Wait for Ctrl+C and signal shutdown.
    """

    loop = asyncio.get_running_loop()

    try:
        loop.add_signal_handler(signal.SIGINT, event.set)
    except NotImplementedError:
        # Windows fallback
        pass

    await event.wait()

    logger.info("Ctrl-C received, signalling shutdown.")


# ------------------------------------------------------------
# Encode Result
# ------------------------------------------------------------

def encode_result(result: OsqueryResult) -> str:

    payload = result.to_dict()

    event = AgentEvent(
        node_id=result.agent_uuid,
        event_type=EventType.OSQUERY,
        payload=payload,
        timestamp_ns=result.timestamp_ns,
        sequence_id=str(uuid.uuid4()),
    )

    return json.dumps(event.to_dict())


# ------------------------------------------------------------
# Helpers
# ------------------------------------------------------------

def hostname_or_default():

    try:
        return socket.gethostname()
    except Exception:
        return "unknown-host"


def read_machine_id():

    paths = [
        "/etc/machine-id",
        "/var/lib/dbus/machine-id",
    ]

    for path in paths:

        try:

            with open(path) as f:

                value = f.read().strip()

                if value:
                    return value

        except Exception:
            continue

    return "unknown-machine-id"


def get_os_version():

    os_release = Path("/etc/os-release")

    if not os_release.exists():
        return platform.platform()

    data = {}

    with open(os_release) as f:

        for line in f:

            line = line.strip()

            if "=" not in line:
                continue

            k, v = line.split("=", 1)

            data[k] = v.strip('"')

    return (
        data.get("PRETTY_NAME")
        or f"{data.get('NAME','Linux')} {data.get('VERSION','')}"
    ).strip()
