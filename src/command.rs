use std::time::Duration;

use anyhow::{Result, anyhow};
use tokio::sync::mpsc;
use tokio::time;
use zenoh::Session;
use zenoh::bytes::{Encoding, ZBytes};

pub type KeyValue = (String, ZBytes);

pub enum Command {
    Get {
        selector: String,
        reply: mpsc::Sender<Result<KeyValue>>,
    },
    Put {
        keyexpr: String,
        payload: String,
        reply: mpsc::Sender<Result<()>>,
    },
    Delete {
        keyexpr: String,
        reply: mpsc::Sender<Result<()>>,
    },
    Subscribe {
        keyexpr: String,
        reply: mpsc::Sender<Result<KeyValue>>,
    },
    Zid {
        reply: mpsc::Sender<Result<String>>,
    },
}

impl Command {
    async fn err(self, err: anyhow::Error) {
        match self {
            Command::Get { reply, .. } => {
                reply.send(Err(err)).await.expect("ui receiver is dropped");
            }
            Command::Put { reply, .. } => {
                reply.send(Err(err)).await.expect("ui receiver is dropped");
            }
            Command::Delete { reply, .. } => {
                reply.send(Err(err)).await.expect("ui receiver is dropped");
            }
            Command::Subscribe { reply, .. } => {
                reply.send(Err(err)).await.expect("ui receiver is dropped");
            }
            Command::Zid { reply, .. } => {
                reply.send(Err(err)).await.expect("ui receiver is dropped");
            }
        }
    }
}

pub fn start_handler(session: Session) -> mpsc::Sender<Command> {
    let (tx, mut rx) = mpsc::channel(8);

    tokio::spawn(async move {
        while let Some(cmd) = rx.recv().await {
            if let Err(err) = handle(&session, &cmd).await {
                cmd.err(err).await;
            };
        }
    });

    tx
}

async fn handle(session: &Session, cmd: &Command) -> Result<()> {
    match cmd {
        Command::Get { selector, reply } => {
            let replies = session
                .get(selector)
                .await
                .map_err(|err| anyhow!("failed to query {selector}: {err}"))?;

            while let Ok(response) = replies.recv_async().await {
                let sample = response.into_result()?;
                reply
                    .send(Ok((
                        sample.key_expr().to_string(),
                        sample.payload().to_owned(),
                    )))
                    .await?;
            }
        }
        Command::Put {
            keyexpr,
            payload,
            reply,
        } => {
            session
                .put(keyexpr, payload)
                .encoding(Encoding::TEXT_PLAIN)
                .await
                .map_err(|err| anyhow!("failed to put {keyexpr}: {err}"))?;
            reply.send(Ok(())).await?;
        }
        Command::Delete { keyexpr, reply } => {
            session
                .delete(keyexpr)
                .await
                .map_err(|err| anyhow!("failed to delete {keyexpr}: {err}"))?;
            reply.send(Ok(())).await?;
        }
        Command::Subscribe { keyexpr, reply } => {
            let subscriber = session
                .declare_subscriber(keyexpr)
                .await
                .map_err(|err| anyhow!("failed to subscribe to {keyexpr}: {err}"))?;

            loop {
                if reply.is_closed() {
                    break;
                }

                match subscriber.try_recv() {
                    Ok(Some(sample)) => {
                        reply
                            .send(Ok((
                                sample.key_expr().to_string(),
                                sample.payload().to_owned(),
                            )))
                            .await?;
                    }
                    Ok(None) => {
                        time::sleep(Duration::from_millis(1)).await;
                        continue;
                    }
                    Err(_) => break,
                }
            }
        }
        Command::Zid { reply } => {
            let zid = session.zid().to_string();
            reply.send(Ok(zid)).await?;
        }
    }

    Ok(())
}
