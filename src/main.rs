use std::env;

use anyhow::{Result, anyhow};
use zenoh::Config;

mod command;
mod ui;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();

    let session = zenoh::open(Config::default())
        .await
        .map_err(|err| anyhow!("failed to create zenoh session: {err}"))?;

    let tx = command::start_handler(session);

    tokio::runtime::Handle::current()
        .spawn_blocking(async move || {
            if args.len() > 0 {
                ui::handle(&tx, args.join(" ").to_string()).await
            } else {
                ui::start(tx).await
            }
        })
        .await?
        .await?;
    Ok(())
}
