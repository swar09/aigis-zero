use clap::Parser;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::interval;

#[derive(Parser, Debug)]
#[command(name = "aigis-zero", version, about = "Aigis-Zero Agent")]
struct Args {
    /// Config path
    #[arg(short, long, default_value = "/etc/aigis-zero/config.toml")]
    config: PathBuf,

    /// Validate config and exit
    #[arg(long)]
    check: bool,

    /// Force re-enrollment
    #[arg(long)]
    enroll: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Parse CLI
    let args = Args::parse();

    // 2. Root check
    if unsafe { libc::getuid() } != 0 {
        eprintln!("Error: aigis-zero must be run as root");
        std::process::exit(1);
    }

    // 3. --check mode
    if args.check {
        let config_str = std::fs::read_to_string(&args.config).map_err(|e| {
            anyhow::anyhow!(
                "Failed to read config file at {}: {}",
                args.config.display(),
                e
            )
        })?;
        // Just parse it as TOML to ensure syntax is valid
        let _parsed: toml::Value = toml::from_str(&config_str)
            .map_err(|e| anyhow::anyhow!("Failed to parse TOML config: {}", e))?;
        println!("Config syntax is valid.");
        std::process::exit(0);
    }

    // Pass config path via env var since orchestrator uses it
    unsafe {
        std::env::set_var("EDR_AGENT_CONFIG", args.config.to_str().unwrap());
    }

    // 6. Install panic hook
    std::panic::set_hook(Box::new(|panic_info| {
        // Log to stderr explicitly in case tracing isn't set up yet
        eprintln!("Agent panicked: {}", panic_info);
        let _ = sd_notify::notify(false, &[sd_notify::NotifyState::Status("Agent panicked")]);
    }));

    // Watchdog task
    tokio::spawn(async {
        let mut ticker = interval(Duration::from_secs(15));
        loop {
            ticker.tick().await;
            let _ = sd_notify::notify(false, &[sd_notify::NotifyState::Watchdog]);
        }
    });

    // Notify ready
    let _ = sd_notify::notify(false, &[sd_notify::NotifyState::Ready]);

    // 10. Call orchestrator
    let res = agent_core::orchestrator::run().await;

    // Notify stopping
    let _ = sd_notify::notify(false, &[sd_notify::NotifyState::Stopping]);

    res
}
