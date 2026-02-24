use std::collections::BTreeSet;

use dhcp_template_crd::{DHCPTemplate, DHCPTemplateStatus, ObjectRef, ObjectRefError};
use kube::api::DynamicObject;
use tracing::{Level, instrument};

use crate::{
    controller::context::Context,
    k8s::{
        discovery::{Discover as _, DiscoveryError},
        dynamic_ext::{
            apply_ext::ApplyError,
            delete_ext::DeleteError,
            owner_ext::{OwnerError, OwnerExt as _},
            safe_ext::{SafeError, SafeExt as _},
        },
    },
};

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum PlanDiffError {
    ObjectRef(#[from] ObjectRefError),
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum PlanExecutionError {
    Discover(#[from] DiscoveryError),
    Owner(#[from] OwnerError),
    Apply(#[from] SafeError<ApplyError>),
    Delete(#[from] SafeError<DeleteError>),
}

#[derive(Debug)]
pub struct Plan<'a> {
    pub apply: BTreeSet<ObjectRef>,
    pub delete: BTreeSet<ObjectRef>,
    manifests: &'a [DynamicObject],
}

impl<'a> Plan<'a> {
    #[instrument(
        skip_all,
        ret(level = Level::DEBUG),
        err(level = Level::WARN),
    )]
    pub fn diff(
        status: Option<&DHCPTemplateStatus>,
        manifests: &'a [DynamicObject],
    ) -> Result<Plan<'a>, PlanDiffError> {
        let pending = status
            .map(|status| status.objects.clone())
            .unwrap_or_default();

        let apply = manifests
            .iter()
            .map(ObjectRef::try_from)
            .collect::<Result<BTreeSet<_>, _>>()?;

        let delete = pending.difference(&apply).cloned().collect();
        let plan = Self {
            apply,
            delete,
            manifests,
        };

        Ok(plan)
    }

    pub fn all(&self) -> BTreeSet<ObjectRef> {
        self.apply.union(&self.delete).cloned().collect()
    }

    pub async fn execute(
        &self,
        object: &DHCPTemplate,
        ctx: &Context,
    ) -> Result<(), PlanExecutionError> {
        for object_ref in &self.delete {
            let api = object_ref.discover(ctx.client()).await?;
            api.safe_delete(&object_ref.name).await?;
        }

        for manifest in self.manifests {
            let mut manifest = manifest.clone();
            manifest.add_owner(object)?;

            let api = manifest.discover(ctx.client()).await?;
            api.safe_apply(&manifest).await?;
        }

        Ok(())
    }
}
