use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use dhcp_template_api::{
    Node, Refresh, Scope, Update,
    controller_service_client::ControllerServiceClient,
    update::Data::{self},
};
use envconfig::Envconfig;
use futures_util::{Stream, TryStreamExt};
use tokio::{select, time::sleep};
use tonic::{Request, transport::Uri};
use tracing::{Level, debug, instrument};

use crate::{provider::Provider, shallow::ShallowClone as _};

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

#[instrument(ret(level = Level::DEBUG))]
fn random_node_name() -> String {
    let r: u64 = rand::random();
    format!("node-{:016x}", r)
}

impl Agent {
    #[instrument(skip_all, fields(node = self.node_name))]
    pub async fn run(&self, provider: Box<dyn Provider>) -> Result<()> {
        let mut updates = self.get_updates(&*provider);

        let mut refresh = Refresh::default();
        let mut update = updates
            .try_next()
            .await?
            .ok_or_else(|| anyhow!("Could not get initial node state."))?;

        loop {
            refresh = self.push_node(&update, refresh.scope()).await?;

            select! {
               _ = sleep(Duration::from_secs(refresh.backoff_seconds)) => {
                   debug!("Backoff duration has passed.");
               }

               result = updates.try_next() => {
                   match result? {
                       Some(next) => {
                           debug!("Interfaces changed.");
                           update = next;
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

    #[instrument(
        skip(self, update),
        fields(token = update.token),
        ret(level = Level::DEBUG),
        err(level = Level::WARN),
    )]
    async fn push_node(&self, update: &Update, scope: Scope) -> Result<Refresh> {
        let mut controller_service = ControllerServiceClient::connect(self.endpoint.clone())
            .await
            .context("Could not connect to controller.")?;

        let request = Request::new(match scope {
            Scope::Shallow => update.shallow_clone(),
            Scope::Full => update.clone(),
        });

        let response = controller_service.push_node(request).await?;
        let refresh = response.into_inner();

        Ok(refresh)
    }

    fn get_updates(&self, provider: &dyn Provider) -> impl Stream<Item = Result<Update>> {
        provider.interfaces().map_ok(|interfaces| Update {
            token: rand::random(),
            data: Some(Data::Full(Node {
                name: self.node_name.clone(),
                interfaces,
            })),
        })
    }
}
