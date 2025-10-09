use std::{env, time::Duration};

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
                // TODO: Scouting seems to be taking some time, and without this sleep we fail to
                // discover any peer and we get no results for our queries in the tests.
                // We should probably connect to testing peers explicitly instead of relying on scouting.
                let _ = tokio::time::sleep(Duration::from_millis(1000)).await;

                ui::handle(&tx, args.join(" ").to_string()).await
            } else {
                ui::start(tx).await
            }
        })
        .await?
        .await?;
    Ok(())
}
