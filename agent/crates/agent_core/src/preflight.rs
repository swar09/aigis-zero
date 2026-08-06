use std::path::Path;
use std::process::Command;

pub struct PreflightReport {
    pub config_dir_writable: Result<(), String>,
    pub data_dir_writable: Result<(), String>,
    pub log_dir_writable: Result<(), String>,
    pub bpf_jit_enabled: Result<bool, String>,
    pub inotify_watches: Result<u64, String>,
    pub osqueryd_installed: Result<String, String>,
    pub nft_installed: Result<String, String>,
    pub is_root: bool,
}

impl PreflightReport {
    pub fn is_ok(&self) -> bool {
        self.config_dir_writable.is_ok()
            && self.data_dir_writable.is_ok()
            && self.log_dir_writable.is_ok()
            && self.osqueryd_installed.is_ok()
            && self.nft_installed.is_ok()
            && self.is_root
    }

    pub fn print(&self) {
        println!("Aigis-Zero Agent Pre-flight Environment Check");

        if self.is_root {
            println!("  [OK]   Running as root (UID 0)");
        } else {
            println!("  [FAIL] Not running as root (required for isolation & raw operations)");
        }

        let print_dir_status = |name: &str, status: &Result<(), String>| match status {
            Ok(_) => println!("  [OK]   {} is accessible and writable", name),
            Err(e) => println!("  [FAIL] {} check failed: {}", name, e),
        };

        print_dir_status("Config Directory", &self.config_dir_writable);
        print_dir_status("Data Directory", &self.data_dir_writable);
        print_dir_status("Log Directory", &self.log_dir_writable);

        match &self.bpf_jit_enabled {
            Ok(true) => println!("  [OK]   BPF JIT compilation is enabled"),
            Ok(false) => {
                println!("  [WARN] BPF JIT compilation is disabled (performance might be affected)")
            }
            Err(e) => println!("  [WARN] Could not verify BPF JIT: {}", e),
        }

        match &self.inotify_watches {
            Ok(val) => {
                if *val >= 524288 {
                    println!(
                        "  [OK]   inotify max_user_watches limit is sufficient ({})",
                        val
                    );
                } else {
                    println!(
                        "  [WARN] inotify max_user_watches limit is low ({}); recommended >= 524288",
                        val
                    );
                }
            }
            Err(e) => println!("  [WARN] Could not verify inotify max_user_watches: {}", e),
        }

        match &self.osqueryd_installed {
            Ok(path) => println!("  [OK]   osqueryd found: {}", path),
            Err(e) => println!("  [FAIL] osqueryd check failed: {}", e),
        }

        match &self.nft_installed {
            Ok(path) => println!("  [OK]   nft (nftables) found: {}", path),
            Err(e) => println!("  [FAIL] nft (nftables) check failed: {}", e),
        }
    }
}

pub fn run_preflight(config: &crate::config::AgentConfig) -> PreflightReport {
    let is_root = unsafe { libc::getuid() } == 0;

    let check_dir_writable = |path: &Path| -> Result<(), String> {
        if !path.exists()
            && let Err(e) = std::fs::create_dir_all(path)
        {
            return Err(format!(
                "Directory does not exist and failed to create: {}",
                e
            ));
        }
        let temp_file = path.join(".aigis_zero_preflight_temp");
        if let Err(e) = std::fs::write(&temp_file, b"test") {
            return Err(format!("Not writable: {}", e));
        }
        let _ = std::fs::remove_file(temp_file);
        Ok(())
    };

    let config_dir = config
        .osquery
        .flags_path
        .parent()
        .unwrap_or_else(|| Path::new("/etc/aigis-zero"));

    let config_dir_writable = check_dir_writable(config_dir);
    let data_dir_writable = check_dir_writable(&config.agent.data_dir);
    let log_dir_writable = check_dir_writable(&config.agent.log_dir);

    let bpf_jit_enabled = std::fs::read_to_string("/proc/sys/net/core/bpf_jit_enable")
        .map(|s| s.trim() == "1")
        .map_err(|e| format!("Failed to read bpf_jit_enable: {}", e));

    let inotify_watches = std::fs::read_to_string("/proc/sys/fs/inotify/max_user_watches")
        .map_err(|e| format!("Failed to read max_user_watches: {}", e))
        .and_then(|s| {
            s.trim()
                .parse::<u64>()
                .map_err(|e| format!("Failed to parse integer: {}", e))
        });

    let osqueryd_installed = if which("osqueryd") {
        Ok("Found in PATH".to_string())
    } else if Path::new("/opt/osquery/bin/osqueryd").exists() {
        Ok("/opt/osquery/bin/osqueryd".to_string())
    } else if Path::new("/usr/bin/osqueryd").exists() {
        Ok("/usr/bin/osqueryd".to_string())
    } else {
        Err("osqueryd executable not found (osquery package is required)".to_string())
    };

    let nft_installed = if which("nft") {
        Ok("Found in PATH".to_string())
    } else if Path::new("/usr/sbin/nft").exists() {
        Ok("/usr/sbin/nft".to_string())
    } else if Path::new("/sbin/nft").exists() {
        Ok("/sbin/nft".to_string())
    } else {
        Err("nft executable not found (nftables is required for isolation)".to_string())
    };

    PreflightReport {
        config_dir_writable,
        data_dir_writable,
        log_dir_writable,
        bpf_jit_enabled,
        inotify_watches,
        osqueryd_installed,
        nft_installed,
        is_root,
    }
}

fn which(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preflight_is_ok_logic() {
        let report = PreflightReport {
            config_dir_writable: Ok(()),
            data_dir_writable: Ok(()),
            log_dir_writable: Ok(()),
            bpf_jit_enabled: Ok(true),
            inotify_watches: Ok(524288),
            osqueryd_installed: Ok("Found".to_string()),
            nft_installed: Ok("Found".to_string()),
            is_root: true,
        };
        assert!(report.is_ok());

        let report_failed = PreflightReport {
            config_dir_writable: Err("fail".to_string()),
            data_dir_writable: Ok(()),
            log_dir_writable: Ok(()),
            bpf_jit_enabled: Ok(true),
            inotify_watches: Ok(524288),
            osqueryd_installed: Ok("Found".to_string()),
            nft_installed: Ok("Found".to_string()),
            is_root: true,
        };
        assert!(!report_failed.is_ok());
    }
}
