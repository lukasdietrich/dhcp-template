use std::{sync::Arc, time::Duration};

use dhcp_template_api::Node;
use moka::future::Cache;

#[derive(Clone)]
pub struct Nodes {
    cache: Cache<String, Arc<Node>>,
}

impl Nodes {
    pub fn init() -> Self {
        Nodes {
            cache: Cache::builder()
                .time_to_idle(Duration::from_secs(60))
                .build(),
        }
    }

    pub async fn needs_refresh(&self, node: &Node) -> bool {
        match self.cache.get(&node.name).await {
            Some(cached) => cached.token != node.token,
            None => true,
        }
    }

    pub async fn insert(&self, node: Node) {
        self.cache.insert(node.name.clone(), Arc::new(node)).await;
    }
}
