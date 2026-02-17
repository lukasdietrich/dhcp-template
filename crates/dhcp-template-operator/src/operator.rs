use std::{sync::Arc, time::Duration};

use dhcp_template_crd::DHCPTemplate;
use futures_util::StreamExt;
use kube::{
    Api, Client, Error,
    runtime::{Controller, controller::Action, watcher::Config},
};
use tracing::{debug, info, warn};

use crate::{state::State, template::ManifestTemplate as _};

#[derive(Clone)]
pub struct Operator {
    state: State,
    client: Client,
}

impl Operator {
    pub async fn new(state: State) -> anyhow::Result<Self> {
        let client = Client::try_default().await?;
        Ok(Self { state, client })
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let templates_api: Api<DHCPTemplate> = Api::all(self.client.clone());

        Controller::new(templates_api, Config::default())
            // TODO .reconcile_all_on(changes)
            .run(reconcile, error_policy, Arc::new(self))
            .for_each(|res| async move {
                match res {
                    Ok(o) => info!("reconciled {:?}", o),
                    Err(e) => warn!("reconcile failed: {:?}", e),
                }
            })
            .await;

        Ok(())
    }
}

async fn reconcile(object: Arc<DHCPTemplate>, operator: Arc<Operator>) -> Result<Action, Error> {
    debug!("{:#?}", object);

    let nodes = operator.state.snapshot();
    let manifests = object.spec.render(nodes);

    debug!("{:#?}", manifests);

    Ok(Action::requeue(Duration::from_secs(10)))
}

fn error_policy(_object: Arc<DHCPTemplate>, _error: &Error, _ctx: Arc<Operator>) -> Action {
    // TODO  Proper error policy with backoff
    Action::await_change()
}
