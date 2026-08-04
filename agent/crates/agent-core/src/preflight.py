import os
import shutil
from dataclasses import dataclass
from pathlib import Path
from typing import Optional

from config import AgentConfig


@dataclass
class PreflightReport:
    config_dir_writable: Optional[str]
    data_dir_writable: Optional[str]
    log_dir_writable: Optional[str]

    bpf_jit_enabled: Optional[bool]
    inotify_watches: Optional[int]

    osqueryd_installed: Optional[str]
    nft_installed: Optional[str]

    is_root: bool

    config_dir_error: Optional[str] = None
    data_dir_error: Optional[str] = None
    log_dir_error: Optional[str] = None

    bpf_jit_error: Optional[str] = None
    inotify_error: Optional[str] = None

    osqueryd_error: Optional[str] = None
    nft_error: Optional[str] = None

    def is_ok(self) -> bool:
        return (
            self.config_dir_error is None
            and self.data_dir_error is None
            and self.log_dir_error is None
            and self.osqueryd_error is None
            and self.nft_error is None
            and self.is_root
        )

    def print(self) -> None:
        print("Aigis-Zero Agent Pre-flight Environment Check")

        if self.is_root:
            print("  [OK]   Running as root (UID 0)")
        else:
            print("  [FAIL] Not running as root (required for isolation & raw operations)")

        self._print_dir_status(
            "Config Directory",
            self.config_dir_error,
        )

        self._print_dir_status(
            "Data Directory",
            self.data_dir_error,
        )

        self._print_dir_status(
            "Log Directory",
            self.log_dir_error,
        )

        if self.bpf_jit_error:
            print(
                f"  [WARN] Could not verify BPF JIT: "
                f"{self.bpf_jit_error}"
            )

        elif self.bpf_jit_enabled:
            print("  [OK]   BPF JIT compilation is enabled")

        else:
            print(
                "  [WARN] BPF JIT compilation is disabled "
                "(performance might be affected)"
            )

        if self.inotify_error:
            print(
                "  [WARN] Could not verify "
                f"inotify max_user_watches: {self.inotify_error}"
            )

        elif self.inotify_watches >= 524288:
            print(
                "  [OK]   inotify max_user_watches "
                f"limit is sufficient ({self.inotify_watches})"
            )

        else:
            print(
                "  [WARN] inotify max_user_watches "
                f"limit is low ({self.inotify_watches}); "
                "recommended >= 524288"
            )

        if self.osqueryd_error:
            print(
                f"  [FAIL] osqueryd check failed: "
                f"{self.osqueryd_error}"
            )
        else:
            print(
                f"  [OK]   osqueryd found: "
                f"{self.osqueryd_installed}"
            )

        if self.nft_error:
            print(
                f"  [FAIL] nft (nftables) check failed: "
                f"{self.nft_error}"
            )
        else:
            print(
                f"  [OK]   nft (nftables) found: "
                f"{self.nft_installed}"
            )

    @staticmethod
    def _print_dir_status(name: str, error: Optional[str]):

        if error is None:
            print(f"  [OK]   {name} is accessible and writable")
        else:
            print(f"  [FAIL] {name} check failed: {error}")


def run_preflight(config: AgentConfig) -> PreflightReport:

    is_root = hasattr(os, "geteuid") and os.geteuid() == 0

    config_dir = (
        Path(config.osquery.flags_path).parent
        if Path(config.osquery.flags_path).parent
        else Path("/etc/aigis-zero")
    )

    config_dir_error = check_dir_writable(config_dir)
    data_dir_error = check_dir_writable(
        Path(config.agent.data_dir)
    )
    log_dir_error = check_dir_writable(
        Path(config.agent.log_dir)
    )

    bpf_jit_enabled = None
    bpf_jit_error = None

    try:
        with open("/proc/sys/net/core/bpf_jit_enable") as f:
            bpf_jit_enabled = f.read().strip() == "1"

    except Exception as e:
        bpf_jit_error = str(e)

    inotify_watches = None
    inotify_error = None

    try:
        with open(
            "/proc/sys/fs/inotify/max_user_watches"
        ) as f:
            inotify_watches = int(f.read().strip())

    except Exception as e:
        inotify_error = str(e)

    osqueryd_path = find_binary(
        "osqueryd",
        [
            "/opt/osquery/bin/osqueryd",
            "/usr/bin/osqueryd",
        ],
    )

    nft_path = find_binary(
        "nft",
        [
            "/usr/sbin/nft",
            "/sbin/nft",
        ],
    )

    return PreflightReport(
        config_dir_writable=str(config_dir),
        data_dir_writable=str(config.agent.data_dir),
        log_dir_writable=str(config.agent.log_dir),

        bpf_jit_enabled=bpf_jit_enabled,
        inotify_watches=inotify_watches,

        osqueryd_installed=osqueryd_path,
        nft_installed=nft_path,

        is_root=is_root,

        config_dir_error=config_dir_error,
        data_dir_error=data_dir_error,
        log_dir_error=log_dir_error,

        bpf_jit_error=bpf_jit_error,
        inotify_error=inotify_error,

        osqueryd_error=(
            None
            if osqueryd_path
            else "osqueryd executable not found (osquery package is required)"
        ),

        nft_error=(
            None
            if nft_path
            else "nft executable not found (nftables is required for isolation)"
        ),
    )


def check_dir_writable(path: Path) -> Optional[str]:

    try:
        path.mkdir(parents=True, exist_ok=True)

        temp = path / ".aigis_zero_preflight_temp"

        temp.write_text("test")

        temp.unlink(missing_ok=True)

        return None

    except Exception as e:
        return str(e)


def find_binary(name: str, fallbacks: list[str]) -> Optional[str]:

    found = shutil.which(name)

    if found:
        return found

    for path in fallbacks:

        if Path(path).exists():
            return path

    return None
