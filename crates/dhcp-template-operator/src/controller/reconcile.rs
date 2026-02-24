use std::{sync::Arc, time::Duration};

use dhcp_template_crd::{DHCPTemplate, Reason, Type};
use kube::{Api, runtime::controller::Action};
use tracing::{Level, info, instrument};

use crate::{
    controller::{
        context::Context,
        plan::{Plan, PlanDiffError, PlanExecutionError},
    },
    k8s::template_ext::{
        condition_ext::DHCPTemplateStatusConditionExt as _, status_ext::StatusError,
    },
    template::{ManifestTemplate as _, TemplateError},
};

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum ReconcileError {
    Template(#[from] TemplateError),
    Status(#[from] StatusError),
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
                .add_condition(
                    &object,
                    Reason::TemplateEvaluation,
                    Type::Error,
                    format!("{err}"),
                )
                .await;

            Err(err)?
        }
    };

    let plan = match Plan::diff(object.status.as_ref(), &manifests) {
        Ok(plan) => plan,
        Err(err) => {
            let _ = api
                .add_condition(
                    &object,
                    Reason::PlanningObjects,
                    Type::Error,
                    format!("{err}"),
                )
                .await;

            Err(err)?
        }
    };

    api.add_condition_with_objects(
        &object,
        plan.all(),
        Reason::Reconciliation,
        Type::Pending,
        "Reconciling template objects.".to_owned(),
    )
    .await?;

    match plan.execute(&object, &ctx).await {
        Ok(()) => {
            api.add_condition_with_objects(
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
            api.add_condition_with_objects(
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
        ReconcileError::Template(_) => Action::await_change(),
        _ => Action::requeue(Duration::from_mins(5)),
    }
}
