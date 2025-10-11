use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use insta_cmd::get_cargo_bin;
use port_check::free_local_port;
use tempdir::TempDir;
use tokio::sync::Mutex;
use zenoh::bytes::ZBytes;

pub type KeyExpr = String;

pub struct Session {
    runtime: tokio::runtime::Runtime,
    session: Arc<zenoh::Session>,
    config_path: PathBuf,
    _temp_dir: TempDir,
}

impl Drop for Session {
    fn drop(&mut self) {
        self.runtime
            .block_on(async { self.session.close().await })
            .unwrap();
    }
}

impl Session {
    pub fn cli(&self) -> Command {
        let mut cmd = Command::new(get_cargo_bin("zenoh-cli"));
        cmd.env("ZENOH_CONFIG", self.config_path.to_owned());
        cmd
    }

    pub fn wait_for_peer(&self) {
        let _guard = self.runtime.enter();
        let fut = tokio::time::timeout(Duration::from_secs(5), async {
            let info = self.session.info();

            loop {
                let mut peers = info.peers_zid().await;
                if peers.next().is_some() {
                    break;
                }

                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });

        self.block_on(fut).expect("failed to wait for peer");
    }

    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        self.runtime.block_on(future)
    }

    pub fn put(&self, key: &str, payload: &str) {
        let key = key.to_string();
        let payload = ZBytes::from(payload);
        self.block_on(async move { self.session.put(key, payload).await })
            .expect("failed to put key");
    }
}

pub struct Builder {
    config: zenoh::Config,
    cli_config: zenoh::Config,
    storage: HashMap<KeyExpr, Storage>,
}

impl Builder {
    fn new() -> Self {
        let port = free_local_port().expect("failed to get a free port");

        let mut config = zenoh::Config::default();
        config
            .insert_json5("mode", r#""router""#)
            .expect("failed to set router mode");
        config
            .insert_json5(
                "listen",
                &format!(
                    r#"{{
            "endpoints": [
                "tcp/127.0.0.1:{port}"
            ]
        }}"#,
                ),
            )
            .expect("failed to set listen config");

        let mut cli_config = zenoh::Config::default();
        cli_config
            .insert_json5("mode", r#""peer""#)
            .expect("failed to set peer mode");
        cli_config
            .insert_json5(
                "connect",
                &format!(
                    r#"{{
    "endpoints": [
        "tcp/127.0.0.1:{port}"
    ]
}}"#
                ),
            )
            .expect("failed to set connect config");
        cli_config
            .insert_json5(
                "scouting",
                r#"{
    "multicast": {
        "enabled": false,
    },
}"#,
            )
            .expect("failed to set scouting config");

        Builder {
            config,
            cli_config,
            storage: HashMap::new(),
        }
    }

    pub fn add_storage(mut self, prefix: impl Into<String>, storage: Storage) -> Self {
        let prefix = prefix.into();
        assert!(!prefix.contains("*"), "prefix must not have wildcards");
        assert!(
            !prefix.ends_with("/"),
            "prefix must not have trailing slashes"
        );
        self.storage.insert(prefix, storage);
        self
    }

    pub fn with_cli_config(mut self, key: &str, value: &str) -> Self {
        self.cli_config
            .insert_json5(key, value)
            .expect("failed to set  config");
        self
    }

    pub fn start(self) -> Session {
        let temp_dir = TempDir::new("zenoht").expect("failed to create tempdir");
        let config_path = temp_dir.path().join("zenoh-conf.json5");
        let mut config_file = File::create(&config_path).expect("failed to create config file");
        serde_json5::to_writer(&mut config_file, &self.cli_config).expect("failed to write config");

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        let session = runtime.block_on(async {
            let session = Arc::new(zenoh::open(self.config).await.unwrap());

            for (prefix, storage) in self.storage {
                let session = session.clone();
                runtime.spawn(async move {
                    storage.clone().handle(prefix, session).await;
                });
            }

            session
        });

        Session {
            runtime,
            session,
            config_path,
            _temp_dir: temp_dir,
        }
    }
}

pub fn builder() -> Builder {
    Builder::new()
}

#[derive(Clone)]
pub struct Storage(Arc<Mutex<HashMap<String, ZBytes>>>);

impl Storage {
    pub fn empty() -> Self {
        Storage(Arc::new(Mutex::new(HashMap::new())))
    }

    pub fn with_entries(entries: &[(&str, &str)]) -> Self {
        Storage(Arc::new(Mutex::new(
            entries
                .iter()
                .map(|(k, v)| (k.to_string(), ZBytes::from(v.as_bytes())))
                .collect(),
        )))
    }

    pub async fn get(&self, key: &str) -> Option<ZBytes> {
        self.0.lock().await.get(key).cloned()
    }

    async fn handle(&self, prefix: String, session: Arc<zenoh::Session>) {
        let prefix = format!("{prefix}/");
        let prefix_with_wildcard = format!("{prefix}**");
        let subscriber = session
            .declare_subscriber(prefix_with_wildcard.clone())
            .await
            .unwrap();
        let queryable = session
            .declare_queryable(prefix_with_wildcard.clone())
            .await
            .unwrap();

        loop {
            if session.is_closed() {
                break;
            }
            tokio::select! {
                sample = subscriber.recv_async() => {
                    if let Ok(sample) = sample {
                        self.handle_sample(&prefix, sample).await;
                    }
                },
                query = queryable.recv_async() => {
                    if let Ok(query) = query {
                        self.handle_query(&prefix, query).await;
                    }
                }
            }
        }
    }

    async fn handle_sample(&self, prefix: &str, sample: zenoh::sample::Sample) {
        let mut storage = self.0.lock().await;
        let key = sample.key_expr().trim_start_matches(prefix).to_string();
        match sample.kind() {
            zenoh::sample::SampleKind::Put => {
                storage.insert(key, sample.payload().to_owned());
            }
            zenoh::sample::SampleKind::Delete => {
                let _ = storage.remove(&key);
            }
        }
    }

    async fn handle_query(&self, prefix: &str, query: zenoh::query::Query) {
        let storage = self.0.lock().await;
        let key = query.key_expr().trim_start_matches(prefix).to_string();
        if let Some(value) = storage.get(&key).cloned() {
            query.reply(query.key_expr(), value).await.unwrap();
        }
    }
}
