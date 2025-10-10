use std::env;

use anyhow::{Result, anyhow};
use zenoh::Config;

mod command;
mod ui;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();

    let config = match env::var("ZENOH_CONFIG") {
        Ok(config) => Config::from_file(config)
            .map_err(|err| anyhow!("failed to load zenoh config: {err}"))?,
        Err(_) => Config::default(),
    };

    let session = zenoh::open(config)
        .await
        .map_err(|err| anyhow!("failed to create zenoh session: {err}"))?;

    let tx = command::start_handler(session);

    tokio::runtime::Handle::current()
        .spawn_blocking(async move || {
            if args.is_empty() {
                ui::start(tx).await
            } else {
                ui::handle(&tx, args.join(" ").to_string()).await
            }
        })
        .await?
        .await?;
    Ok(())
}
