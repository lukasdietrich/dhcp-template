use std::{sync::Arc, time::Duration};

use dhcp_template_crd::DHCPTemplate;
use futures_time::stream::StreamExt as _;
use futures_util::StreamExt;
use kube::{
    Api, Client,
    runtime::{Controller, controller::Action, watcher::Config},
};
use tracing::{Level, instrument, warn};

use crate::{
    api_ext::{ApiExt as _, ApplyError, OwnerExt as _, OwnerRefError},
    discovery::{Discover as _, DiscoverError},
    state::State,
    template::{ManifestTemplate as _, TemplateError},
};

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
        let config = Config::default();

        let state_changes = self
            .state
            .changes()
            .debounce(futures_time::time::Duration::from(Duration::from_secs(10)));

        Controller::new(templates_api, config)
            .reconcile_all_on(state_changes)
            .run(reconcile, error_policy, Arc::new(self))
            .for_each(|res| async move {
                if let Err(error) = res {
                    warn!("Reconciliation failed: {}", error);
                }
            })
            .await;

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
enum ReconcileError {
    #[error("Invalid configuration: {0}")]
    Configuration(#[from] TemplateError),

    #[error("Could not discover api: {0}")]
    DiscoverError(#[from] DiscoverError),

    #[error(transparent)]
    KubeError(#[from] kube::Error),

    #[error("Could not apply manifest: {0}")]
    ApplyError(#[from] ApplyError),

    #[error("Could not take owner ref: {0}")]
    OwnerRefError(#[from] OwnerRefError),
}

#[instrument(
    skip_all,
    fields(name = object.metadata.name, namespace = object.metadata.namespace),
    ret(level = Level::DEBUG),
    err(level = Level::WARN)
)]
async fn reconcile(
    object: Arc<DHCPTemplate>,
    operator: Arc<Operator>,
) -> Result<Action, ReconcileError> {
    // let template_api = object.discover(&operator.client).await?;

    let nodes = operator.state.snapshot();
    let manifests = object.spec.render(nodes)?;

    for mut manifest in manifests {
        manifest.add_owner(&object)?;

        let api = manifest.discover(&operator.client).await?;
        api.apply(&manifest).await?;
    }

    Ok(Action::requeue(Duration::from_hours(1)))
}

#[instrument(
    skip_all,
    fields(name = object.metadata.name, namespace = object.metadata.namespace),
    ret(level = Level::DEBUG),
)]
fn error_policy(
    object: Arc<DHCPTemplate>,
    error: &ReconcileError,
    _operator: Arc<Operator>,
) -> Action {
    match error {
        ReconcileError::Configuration(_) => Action::await_change(),
        ReconcileError::KubeError(_) => Action::requeue(Duration::from_mins(1)),
        _ => Action::requeue(Duration::from_mins(5)),
    }
}
