use std::{sync::Arc, time::Duration};

use dhcp_template_crd::{DHCPTemplate, ObjectRefError, Reason, Type};
use kube::{Api, runtime::controller::Action};
use tracing::{Level, info, instrument};

use crate::{
    controller::{
        context::Context,
        plan::{Plan, PlanDiffError, PlanExecutionError},
        status::DHCPTemplateStatusExt as _,
    },
    k8s::{
        api_ext::{ApiExtError, OwnerRefError},
        discovery::DiscoverError,
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

    PlanDiff(#[from] PlanDiffError),
    PlanExecution(#[from] PlanExecutionError),
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
    let api: Api<DHCPTemplate> = Api::all(ctx.client());
    let nodes = ctx.snapshot();

    if nodes.is_empty() {
        info!("Skipping reconciliation, because no nodes have been registered yet.");
        return Ok(Action::await_change());
    }

    let manifests = match object.spec.render(nodes) {
        Ok(manifests) => manifests,
        Err(err) => {
            let _ = api
                .set_template_error(&object, Reason::TemplateEvaluation, format!("{err}"))
                .await;

            Err(err)?
        }
    };

    let plan = match Plan::diff(object.status.as_ref(), &manifests) {
        Ok(plan) => plan,
        Err(err) => {
            let _ = api
                .set_template_error(&object, Reason::PlanningObjects, format!("{err}"))
                .await;

            Err(err)?
        }
    };

    api.set_template_status(
        &object,
        plan.all(),
        Reason::Reconciliation,
        Type::Pending,
        "Reconciling template objects.".to_owned(),
    )
    .await?;

    match plan.execute(&object, &ctx).await {
        Ok(()) => {
            api.set_template_status(
                &object,
                plan.apply,
                Reason::AllObjectsReady,
                Type::Ready,
                "Template objects reconciled successfully.".to_owned(),
            )
            .await?;

            Ok(Action::requeue(Duration::from_hours(6)))
        }

        Err(err) => {
            api.set_template_status(
                &object,
                plan.all(),
                Reason::Reconciliation,
                Type::Error,
                format!("{err}"),
            )
            .await?;
            Err(err.into())
        }
    }
}

#[instrument(
    skip_all,
    fields(name = object.metadata.name, namespace = object.metadata.namespace),
    ret(level = Level::DEBUG),
)]
#[allow(clippy::needless_pass_by_value)]
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
