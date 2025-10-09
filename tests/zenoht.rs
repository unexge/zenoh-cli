use std::time::Duration;

use tokio;
use zenoh;

pub struct Session {
    _runtime: tokio::runtime::Runtime,
    _session: zenoh::Session,
}

pub struct Builder {
    config: zenoh::Config,
    queryable_key_values: Vec<(String, String)>,
}

impl Builder {
    fn new() -> Self {
        Builder {
            config: zenoh::Config::default(),
            queryable_key_values: Vec::new(),
        }
    }

    pub fn with_queryable_key_value(mut self, key: &str, value: &str) -> Self {
        self.queryable_key_values
            .push((key.to_string(), value.to_string()));
        self
    }

    pub fn start(self) -> Session {
        let runtime = tokio::runtime::Builder::new_multi_thread().build().unwrap();

        let session = runtime.block_on(async {
            let session = zenoh::open(self.config).await.unwrap();

            for (key, value) in self.queryable_key_values {
                let queryable = session.declare_queryable(key.clone()).await.unwrap();
                runtime.spawn(async move {
                    while let Ok(query) = queryable.recv_async().await {
                        query.reply(key.clone(), value.clone()).await.unwrap();
                    }
                });
            }

            session
        });

        std::thread::sleep(Duration::from_secs(1));

        Session {
            _runtime: runtime,
            _session: session,
        }
    }
}

pub fn builder() -> Builder {
    Builder::new()
}
