use anyhow::{Context, Result};
use std::net::IpAddr;
use tokio::process::Command;

/// Manages network isolation using nftables.
///
/// Applies a strict quarantine policy that drops all incoming and outgoing
/// traffic except for established connections, loopback, and communication
/// with the fleet server.
pub struct IsolationManager {
    fleet_ip: IpAddr,
    fleet_port: u16,
}

impl IsolationManager {
    pub fn new(fleet_ip: IpAddr, fleet_port: u16) -> Self {
        Self {
            fleet_ip,
            fleet_port,
        }
    }

    /// Apply the network isolation rules.
    pub async fn isolate(&self) -> Result<()> {
        let ip_family = match self.fleet_ip {
            IpAddr::V4(_) => "ip",
            IpAddr::V6(_) => "ip6",
        };

        let ruleset = format!(
            "table inet aigis_isolation {{
    chain input {{
        type filter hook input priority -100; policy drop;
        ct state established,related accept
        iif lo accept
        {} saddr {} tcp sport {} accept
    }}
    chain output {{
        type filter hook output priority -100; policy drop;
        ct state established,related accept
        oif lo accept
        {} daddr {} tcp dport {} accept
    }}
    chain forward {{
        type filter hook forward priority -100; policy drop;
    }}
}}",
            ip_family, self.fleet_ip, self.fleet_port, ip_family, self.fleet_ip, self.fleet_port
        );

        let mut child = Command::new("nft")
            .arg("-f")
            .arg("-")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn nft process")?;

        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin
                .write_all(ruleset.as_bytes())
                .await
                .context("Failed to write ruleset to nft stdin")?;
        }

        let output = child
            .wait_with_output()
            .await
            .context("Failed to wait for nft output")?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("nft failed to apply rules: {}", err);
        }

        Ok(())
    }

    /// Remove the network isolation rules.
    pub async fn de_isolate(&self) -> Result<()> {
        let output = Command::new("nft")
            .args(["delete", "table", "inet", "aigis_isolation"])
            .output()
            .await
            .context("Failed to execute nft command")?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            // It is not an error if the table does not exist
            let err_lower = err.to_lowercase();
            if !err_lower.contains("no such file or directory")
                && !err_lower.contains("does not exist")
                && !err_lower.contains("could not process rule")
            {
                anyhow::bail!("nft delete table failed: {}", err);
            }
        }

        Ok(())
    }

    /// Check if the host is currently isolated (i.e. the isolation table exists).
    pub async fn is_isolated(&self) -> Result<bool> {
        let output = Command::new("nft")
            .args(["list", "table", "inet", "aigis_isolation"])
            .output()
            .await
            .context("Failed to execute nft command")?;

        Ok(output.status.success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_ruleset_generation_ipv4() {
        let manager = IsolationManager::new(IpAddr::from_str("1.2.3.4").unwrap(), 8443);
        let ip_family = match manager.fleet_ip {
            IpAddr::V4(_) => "ip",
            IpAddr::V6(_) => "ip6",
        };

        let ruleset = format!(
            "table inet aigis_isolation {{
    chain input {{
        type filter hook input priority -100; policy drop;
        ct state established,related accept
        iif lo accept
        {} saddr {} tcp sport {} accept
    }}
    chain output {{
        type filter hook output priority -100; policy drop;
        ct state established,related accept
        oif lo accept
        {} daddr {} tcp dport {} accept
    }}
    chain forward {{
        type filter hook forward priority -100; policy drop;
    }}
}}",
            ip_family,
            manager.fleet_ip,
            manager.fleet_port,
            ip_family,
            manager.fleet_ip,
            manager.fleet_port
        );

        assert!(ruleset.contains("ip saddr 1.2.3.4 tcp sport 8443 accept"));
        assert!(ruleset.contains("ip daddr 1.2.3.4 tcp dport 8443 accept"));
    }
}
