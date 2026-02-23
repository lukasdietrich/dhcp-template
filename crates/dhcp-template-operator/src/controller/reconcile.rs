use std::{sync::Arc, time::Duration};

use dhcp_template_crd::{DHCPTemplate, ObjectRefError};
use kube::{Api, api::DeleteParams, runtime::controller::Action};
use tracing::{Level, info, instrument, warn};

use crate::{
    controller::{context::Context, plan::Plan, status::DHCPTemplateStatusExt as _},
    k8s::{
        api_ext::{ApiExt as _, ApiExtError, OwnerExt as _, OwnerRefError},
        discovery::{Discover as _, DiscoverError},
    },
    template::{ManifestTemplate as _, TemplateError},
};

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum ReconcileError {
    Configuration(#[from] TemplateError),
    DiscoverError(#[from] DiscoverError),
    KubeError(#[from] kube::Error),
    ApiError(#[from] ApiExtError),
    OwnerRefError(#[from] OwnerRefError),
    ObjectRefError(#[from] ObjectRefError),
}

#[instrument(
    skip_all,
    fields(name = object.metadata.name, namespace = object.metadata.namespace),
    ret(level = Level::DEBUG),
    err(level = Level::WARN)
)]
pub async fn reconcile(
    object: Arc<DHCPTemplate>,
    ctx: Arc<Context>,
) -> Result<Action, ReconcileError> {
    let nodes = ctx.snapshot();
    if nodes.is_empty() {
        info!("Skipping reconciliation, because no nodes have been registered yet.");
        return Ok(Action::await_change());
    }

    let manifests = object.spec.render(nodes)?;
    let plan = Plan::diff(&object.status, &manifests)?;

    let template_api: Api<DHCPTemplate> = Api::all(ctx.client());
    template_api.set_status_pending(&object, plan.all()).await?;

    for object_ref in &plan.delete {
        let api = object_ref.discover(ctx.client()).await?;
        let res = api.delete(&object_ref.name, &DeleteParams::default()).await;

        match res {
            Err(kube::Error::Api(status)) if status.is_not_found() => {
                warn!("Could not delete object.");
            }
            _ => {
                res?;
            }
        };
    }

    for mut manifest in manifests {
        manifest.add_owner(&object)?;

        let api = manifest.discover(ctx.client()).await?;
        api.apply(&manifest).await?;
    }

    template_api.set_status_ready(&object, plan.apply).await?;
    Ok(Action::requeue(Duration::from_hours(1)))
}

#[instrument(
    skip_all,
    fields(name = object.metadata.name, namespace = object.metadata.namespace),
    ret(level = Level::DEBUG),
)]
pub fn error_policy(
    object: Arc<DHCPTemplate>,
    error: &ReconcileError,
    _ctx: Arc<Context>,
) -> Action {
    match error {
        ReconcileError::Configuration(_) => Action::await_change(),
        ReconcileError::KubeError(_) => Action::requeue(Duration::from_mins(1)),
        _ => Action::requeue(Duration::from_mins(5)),
    }
}
