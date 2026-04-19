use oblivion_shell::Shell;
use anyhow::Result;
use tracing::info;

fn main() -> Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();

    info!("Starting OblivionOS Shell v{}", env!("CARGO_PKG_VERSION"));

    let shell = Shell::new()?;

    info!("OblivionOS Shell initialized");

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}