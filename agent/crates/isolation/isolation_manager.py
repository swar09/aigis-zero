import asyncio
import ipaddress


class IsolationManager:
    """
    Manages network isolation using nftables.

    Applies a strict quarantine policy that drops all incoming and outgoing
    traffic except for:
      - established/related connections
      - loopback traffic
      - communication with the fleet server
    """

    def __init__(self, fleet_ip: str, fleet_port: int):
        self.fleet_ip = ipaddress.ip_address(fleet_ip)
        self.fleet_port = fleet_port


    def _build_ruleset(self):
        ip_family = "ip" if self.fleet_ip.version == 4 else "ip6"

        return f"""table inet aigis_isolation {{
    chain input {{
        type filter hook input priority -100; policy drop;
        ct state established,related accept
        iif lo accept
        {ip_family} saddr {self.fleet_ip} tcp sport {self.fleet_port} accept
    }}
    chain output {{
        type filter hook output priority -100; policy drop;
        ct state established,related accept
        oif lo accept
        {ip_family} daddr {self.fleet_ip} tcp dport {self.fleet_port} accept
    }}
    chain forward {{
        type filter hook forward priority -100; policy drop;
    }}
}}"""

    async def isolate(self) -> None:
        """
        Apply the network isolation rules.
        """

        ip_family = "ip" if self.fleet_ip.version == 4 else "ip6"

        ruleset = self._build_ruleset()

        try:
            process = await asyncio.create_subprocess_exec(
                "nft",
                "-f",
                "-",
                stdin=asyncio.subprocess.PIPE,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
        except Exception as e:
            raise RuntimeError(f"Failed to spawn nft process: {e}") from e

        _, stderr = await process.communicate(ruleset.encode())

        if process.returncode != 0:
            raise RuntimeError(
                f"nft failed to apply rules: {stderr.decode().strip()}"
            )

    async def de_isolate(self) -> None:
        """
        Remove the network isolation rules.
        """

        try:
            process = await asyncio.create_subprocess_exec(
                "nft",
                "delete",
                "table",
                "inet",
                "aigis_isolation",
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
        except Exception as e:
            raise RuntimeError(f"Failed to execute nft command: {e}") from e

        _, stderr = await process.communicate()

        if process.returncode != 0:
            err = stderr.decode().lower()

            # Ignore errors if the table doesn't exist
            if (
                "no such file or directory" not in err
                and "does not exist" not in err
                and "could not process rule" not in err
            ):
                raise RuntimeError(f"nft delete table failed: {stderr.decode().strip()}")

    async def is_isolated(self) -> bool:
        """
        Check whether the isolation table currently exists.
        """

        try:
            process = await asyncio.create_subprocess_exec(
                "nft",
                "list",
                "table",
                "inet",
                "aigis_isolation",
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
        except Exception as e:
            raise RuntimeError(f"Failed to execute nft command: {e}") from e

        await process.communicate()

        return process.returncode == 0
