use std::collections::BTreeSet;

use dhcp_template_crd::{DHCPTemplate, DHCPTemplateStatus, ObjectRef, ObjectRefError};
use kube::api::{DeleteParams, DynamicObject};
use tracing::{Level, instrument, warn};

use crate::{
    controller::context::Context,
    k8s::{
        api_ext::{ApiExt as _, ApiExtError, OwnerExt as _, OwnerRefError},
        discovery::{Discover as _, DiscoverError},
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
    Discover(#[from] DiscoverError),
    Kube(#[from] kube::Error),
    ApiExt(#[from] ApiExtError),
    OwnerRef(#[from] OwnerRefError),
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

        for manifest in self.manifests {
            let mut manifest = manifest.clone();
            manifest.add_owner(object)?;

            let api = manifest.discover(ctx.client()).await?;
            api.apply(&manifest).await?;
        }

        Ok(())
    }
}
