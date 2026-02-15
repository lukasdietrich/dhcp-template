use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use dhcp_template_api::{Node, Refresh, Scope, controller_service_client::ControllerServiceClient};
use envconfig::Envconfig;
use futures_util::{TryStream, TryStreamExt};
use log::debug;
use tokio::{select, time::sleep};
use tonic::{Request, transport::Uri};

use crate::provider::Provider;

#[derive(Debug, Envconfig)]
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
    pub async fn run(&self, provider: Box<dyn Provider>) -> Result<()> {
        let mut updates = self.get_node(&*provider);

        let mut refresh = Refresh::default();
        let mut node = updates
            .try_next()
            .await?
            .ok_or_else(|| anyhow!("Could not get initial node state."))?;

        loop {
            debug!("Sending current {} state.", refresh.scope().as_str_name());
            refresh = self.push_node(&node, refresh.scope()).await?;

            select! {
               _ = sleep(Duration::from_secs(refresh.backoff_seconds)) => {
                   debug!("Backoff duration of {}s has passed.", refresh.backoff_seconds);
               }

               result = updates.try_next() => {
                   match result? {
                       Some(next) => {
                           debug!("Interfaces changed.");
                           node = next;
                           refresh.set_scope(Scope::Full);
                       },
                       None => {
                           bail!("Provider closed!");
                       },
                   }
               }
            }
        }
    }

    async fn push_node(&self, node: &Node, scope: Scope) -> Result<Refresh> {
        let mut controller_service = ControllerServiceClient::connect(self.endpoint.clone())
            .await
            .context("Could not connect to controller.")?;

        let request = Request::new(match scope {
            Scope::Shallow => shallow_clone(&node),
            Scope::Full => node.clone(),
        });

        let response = controller_service.push_node(request).await?;
        let refresh = response.into_inner();

        Ok(refresh)
    }

    fn get_node(
        &self,
        provider: &dyn Provider,
    ) -> impl TryStream<Ok = Node, Error = anyhow::Error> {
        provider.interfaces().map_ok(|interfaces| Node {
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
