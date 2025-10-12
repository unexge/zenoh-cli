use anyhow::{Result, bail};
use colored::Colorize;
use rustyline::{Config, DefaultEditor, error::ReadlineError};
use tokio::signal;
use tokio::sync::mpsc;

use super::command::{Command, KeyValue};

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub async fn start(commands: mpsc::Sender<Command>) -> Result<()> {
    println!("Zenoh CLI v{VERSION}");
    let mut rl = DefaultEditor::with_config(Config::builder().auto_add_history(true).build())?;

    for input in rl.iter("> ") {
        let input = match input {
            Ok(input) => input,
            Err(ReadlineError::Eof | ReadlineError::Interrupted) => break,
            Err(err) => bail!("failed to read input: {err}"),
        };

        if let Err(err) = handle(&commands, input).await {
            if err.downcast_ref::<Exit>().is_some() {
                break;
            }

            println!("{}", format!("error: {err}").red());
        }
    }

    Ok(())
}

pub async fn handle(commands: &mpsc::Sender<Command>, input: String) -> Result<()> {
    let mut input = input.trim().split(" ");
    match input.next().unwrap_or_default() {
        "q" | "quit" => bail!(Exit),
        "get" => {
            let Some(selector) = input.next().map(str::to_string) else {
                bail!("missing selector");
            };

            let (tx, mut rx) = mpsc::channel(8);
            if let Err(err) = commands
                .send(Command::Get {
                    selector,
                    reply: tx,
                })
                .await
            {
                bail!("failed to send command: {err}");
            }

            let mut num_replies = 0;
            while let Some(res) = rx.recv().await {
                print_key_value(res?);
                num_replies += 1;
            }
            if num_replies == 0 {
                println!("{}", "no replies received".bright_black());
            }
        }
        "put" => {
            let Some(keyexpr) = input.next().map(str::to_string) else {
                bail!("missing key expression");
            };
            let Some(payload) = input.next().map(str::to_string) else {
                bail!("missing payload");
            };

            let (tx, mut rx) = mpsc::channel(1);
            if let Err(err) = commands
                .send(Command::Put {
                    keyexpr,
                    payload,
                    reply: tx,
                })
                .await
            {
                bail!("failed to send command: {err}");
            }

            match rx.recv().await {
                Some(Ok(())) => {
                    println!("{}", "ok".bright_black())
                }
                Some(Err(err)) => {
                    bail!(err);
                }
                None => {
                    bail!("failed to write keyexpr");
                }
            }
        }
        "delete" | "del" => {
            let Some(keyexpr) = input.next().map(str::to_string) else {
                bail!("missing key expression");
            };

            let (tx, mut rx) = mpsc::channel(1);
            if let Err(err) = commands.send(Command::Delete { keyexpr, reply: tx }).await {
                bail!("failed to send command: {err}");
            }

            match rx.recv().await {
                Some(Ok(())) => {
                    println!("{}", "ok".bright_black())
                }
                Some(Err(err)) => {
                    bail!(err);
                }
                None => {
                    bail!("failed to delete keyexpr");
                }
            }
        }
        "subscribe" | "sub" => {
            let Some(keyexpr) = input.next().map(str::to_string) else {
                bail!("missing key expression");
            };

            let (tx, mut rx) = mpsc::channel(8);
            if let Err(err) = commands
                .send(Command::Subscribe { keyexpr, reply: tx })
                .await
            {
                bail!("failed to send command: {err}");
            }

            loop {
                tokio::select! {
                    res = rx.recv() => match res {
                        Some(res) => {
                            print_key_value(res?);
                        }
                        None => {
                            break;
                        }
                    },
                    _ = signal::ctrl_c() => {
                        break;
                    }
                }
            }
        }
        "zid" => {
            let (tx, mut rx) = mpsc::channel(1);
            if let Err(err) = commands.send(Command::Zid { reply: tx }).await {
                bail!("failed to send command: {err}");
            }

            match rx.recv().await {
                Some(Ok(zid)) => {
                    println!("{zid}")
                }
                Some(Err(err)) => {
                    bail!(err);
                }
                None => {
                    bail!("failed to get zid");
                }
            }
        }
        "peers" => {
            let (tx, mut rx) = mpsc::channel(8);
            if let Err(err) = commands.send(Command::Peers { reply: tx }).await {
                bail!("failed to send command: {err}");
            }

            let mut num_replies = 0;
            while let Some(res) = rx.recv().await {
                println!("{}", res?);
                num_replies += 1;
            }
            if num_replies == 0 {
                println!("{}", "no peers found".bright_black());
            }
        }
        "routers" => {
            let (tx, mut rx) = mpsc::channel(8);
            if let Err(err) = commands.send(Command::Routers { reply: tx }).await {
                bail!("failed to send command: {err}");
            }

            let mut num_replies = 0;
            while let Some(res) = rx.recv().await {
                println!("{}", res?);
                num_replies += 1;
            }
            if num_replies == 0 {
                println!("{}", "no routers found".bright_black());
            }
        }
        cmd => {
            if cmd.is_empty() {
                bail!("missing command");
            }
            bail!("unknown command: {cmd}");
        }
    }

    Ok(())
}

fn print_key_value((keyexpr, value): KeyValue) {
    println!(
        "{}: {}",
        keyexpr.bright_black(),
        value.try_to_string().expect("value must be utf-8")
    );
}

#[derive(Debug, Clone)]
struct Exit;

impl std::fmt::Display for Exit {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "exit")
    }
}
