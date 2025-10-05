use anyhow::{Result, anyhow};
use zenoh::Config;

mod command;
mod ui;

#[tokio::main]
async fn main() -> Result<()> {
    let session = zenoh::open(Config::default())
        .await
        .map_err(|err| anyhow!("failed to create zenoh session: {err}"))?;

    let tx = command::start_handler(session);
    tokio::runtime::Handle::current()
        .spawn_blocking(|| ui::start(tx))
        .await?
        .await?;
    Ok(())
}
