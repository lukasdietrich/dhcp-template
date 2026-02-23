use std::{cmp::max, sync::Arc, time::Duration};

use dhcp_template_api::{Node, Shallow};
use envconfig::Envconfig;
use futures_util::{Stream, StreamExt as _};
use itertools::Itertools as _;
use moka::future::Cache;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tracing::{Level, error, instrument};

#[derive(Debug, Envconfig)]
pub struct Config {
    #[envconfig(from = "DHCP_TEMPLATE__STATE_IDLE_SECONDS", default = "60")]
    idle_seconds: u64,
}

#[derive(Clone)]
pub struct State {
    nodes: Cache<String, (Arc<Node>, u64)>,
    refresh_seconds: u64,
    notifier: broadcast::Sender<()>,
}

impl From<Config> for State {
    fn from(config: Config) -> Self {
        let (notifier, _) = broadcast::channel(64);
        let eviction = notifier.clone();

        let nodes = Cache::builder()
            .time_to_idle(Duration::from_secs(config.idle_seconds))
            .eviction_listener(move |_key, _value, _cause| {
                if let Err(err) = eviction.send(()) {
                    error!("Could not send state eviction event: {}.", err);
                }
            })
            .build();

        let refresh_seconds = max(config.idle_seconds / 2, 5);

        Self {
            nodes,
            refresh_seconds,
            notifier,
        }
    }
}

#[derive(Debug)]
pub enum Status {
    Ok(u64),
    Deprecated,
    Unknown,
}

impl State {
    #[instrument(skip_all, fields(node = node.name), ret(level = Level::DEBUG))]
    pub async fn status(&self, (node, token): (&Shallow, u64)) -> Status {
        match self.nodes.get(&node.name).await {
            Some((_, cached_token)) if cached_token == token => Status::Ok(self.refresh_seconds),
            Some(_) => Status::Deprecated,
            None => Status::Unknown,
        }
    }

    #[instrument(skip_all, fields(node = node.name), ret(level = Level::DEBUG))]
    pub async fn insert(&self, (node, token): (&Node, u64)) -> Status {
        self.nodes
            .insert(node.name.clone(), (Arc::new(node.clone()), token))
            .await;

        if let Err(err) = self.notifier.send(()) {
            error!("Could not send state change event: {}.", err);
        }

        Status::Ok(self.refresh_seconds)
    }

    pub fn snapshot(&self) -> Vec<Arc<Node>> {
        self.nodes
            .iter()
            .map(|(_, (node, _))| node)
            .sorted_by(|a, b| a.name.cmp(&b.name))
            .collect()
    }

    pub fn changes(&self) -> impl Stream<Item = ()> + use<> {
        BroadcastStream::new(self.notifier.subscribe()).filter_map(async |res| res.ok())
    }
}
