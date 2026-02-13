use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use dhcp_template_api::{Node, Scope, controller_service_client::ControllerServiceClient};
use dhcp_template_stream::result::ResultStreamExt;
use envconfig::Envconfig;
use log::debug;
use tokio::{select, time::sleep};
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, transport::Uri};

use crate::provider::Provider;

#[derive(Envconfig)]
pub struct Config {
    #[envconfig(from = "DHCP_TEMPLATE__NODE_NAME")]
    node_name: Option<String>,

    #[envconfig(from = "DHCP_TEMPLATE__ENDPOINT", default = "http://[::1]:50051")]
    endpoint: Uri,
}

pub struct Agent {
    node_name: String,
    endpoint: Uri,
}

impl From<Config> for Agent {
    fn from(config: Config) -> Self {
        Self {
            node_name: config.node_name.unwrap_or_else(random_node_name),
            endpoint: config.endpoint,
        }
    }
}

fn random_node_name() -> String {
    let r: u64 = rand::random();
    let name = format!("node-{:016x}", r);

    debug!(
        "No node name provided, using generated name to avoid conflicts {}.",
        name
    );

    name
}

impl Agent {
    pub async fn push_node(&self, provider: Box<dyn Provider>) -> Result<()> {
        let mut controller_service = ControllerServiceClient::connect(self.endpoint.clone())
            .await
            .context("Could not connect to controller.")?;

        let mut node_stream = self.get_node(&*provider);

        let mut scope = Scope::Full;
        let mut node = node_stream
            .try_next()
            .await?
            .ok_or_else(|| anyhow!("Could not get initial node state."))?;

        loop {
            debug!("Sending current state: Scope = {:?}.", scope);

            let request = Request::new(match scope {
                Scope::Shallow => shallow_clone(&node),
                Scope::Full => node.clone(),
            });

            let response = controller_service.push_node(request).await?;
            let refresh = response.into_inner();

            scope = refresh.scope();

            select! {
               _ = sleep(Duration::from_secs(refresh.backoff_seconds)) => {
                   debug!("Backoff duration has passed: {}s", refresh.backoff_seconds);
               }

               result = node_stream.try_next() => {
                   match result? {
                       Some(next) => {
                           debug!("Received new state.");
                           scope = Scope::Full;
                           node = next;
                       },
                       None => {
                           bail!("Provider closed!");
                       },
                   }
               }
            }
        }
    }

    fn get_node(&self, provider: &dyn Provider) -> impl Stream<Item = Result<Node>> {
        provider.interfaces().try_map(|interfaces| Node {
            name: self.node_name.clone(),
            scope: Scope::Full.into(),
            token: rand::random(),
            interfaces,
        })
    }
}

fn shallow_clone(node: &Node) -> Node {
    Node {
        name: node.name.clone(),
        scope: Scope::Shallow.into(),
        token: node.token,
        interfaces: Default::default(),
    }
}
