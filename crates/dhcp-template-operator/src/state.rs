use std::{cmp::max, sync::Arc, time::Duration};

use dhcp_template_api::{Node, Shallow};
use envconfig::Envconfig;
use log::debug;
use moka::future::Cache;

#[derive(Debug, Envconfig)]
pub struct Config {
    #[envconfig(from = "DHCP_TEMPLATE__STATE_IDLE_SECONDS", default = "60")]
    idle_seconds: u64,
}

#[derive(Clone)]
pub struct State {
    nodes: Cache<String, (Arc<Node>, u64)>,
    refresh_seconds: u64,
}

impl From<Config> for State {
    fn from(config: Config) -> Self {
        let nodes = Cache::builder()
            .time_to_idle(Duration::from_secs(config.idle_seconds))
            .build();

        let refresh_seconds = max(config.idle_seconds / 2, 5);

        Self {
            nodes,
            refresh_seconds,
        }
    }
}

pub enum Status {
    Ok(u64),
    Deprecated,
    Unknown,
}

impl State {
    pub async fn status(&self, (node, token): (&Shallow, u64)) -> Status {
        match self.nodes.get(&node.name).await {
            Some((_, cached_token)) if cached_token == token => {
                debug!(
                    "Node {} is still valid, asking for refresh in {}s.",
                    node.name, self.refresh_seconds
                );
                Status::Ok(self.refresh_seconds)
            }
            Some(_) => {
                debug!("Node {} is deprecated.", node.name);
                Status::Deprecated
            }
            None => {
                debug!("Node {} is unknown.", node.name);
                Status::Unknown
            }
        }
    }

    pub async fn insert(&self, (node, token): (&Node, u64)) -> Status {
        debug!(
            "Updating node state for {}, asking for refresh in {}s.",
            node.name, self.refresh_seconds
        );

        self.nodes
            .insert(node.name.clone(), (Arc::new(node.clone()), token))
            .await;

        Status::Ok(self.refresh_seconds)
    }

    pub fn snapshot(&self) -> Vec<Node> {
        let mut nodes: Vec<Node> = self
            .nodes
            .iter()
            .map(|(_, (node, _))| (*node).clone())
            .collect();

        nodes.sort_by(|a, b| a.name.cmp(&b.name));
        nodes
    }
}
