use std::{fmt::Debug, ops::Deref, sync::Arc};

use dhcp_template_crd::{DHCPTemplate, DHCPTemplateStatus};
use itertools::Itertools as _;
use kube::{
    Api, Resource, ResourceExt as _,
    api::{DynamicObject, Patch, PatchParams, PostParams},
    runtime::reflector::Lookup,
};
use serde::{Serialize, de::DeserializeOwned};

#[derive(Debug, thiserror::Error)]
pub enum ApiExtError {
    #[error(transparent)]
    KubeError(#[from] kube::Error),

    #[error("Missing resource name.")]
    ResourceName,
}

pub trait ApiExt<K> {
    type Error;

    async fn apply(&self, patch: &K) -> Result<K, Self::Error>;
}

impl<K> ApiExt<K> for Api<K>
where
    K: Clone + Debug + Serialize + DeserializeOwned + Lookup,
{
    type Error = ApiExtError;

    async fn apply(&self, patch: &K) -> Result<K, Self::Error> {
        let name = patch.name().ok_or(ApiExtError::ResourceName)?;
        let params = PatchParams::apply("dhcp-template-operator");
        let patch = Patch::Apply(patch);

        Ok(self.patch::<_>(&name, &params, &patch).await?)
    }
}

pub trait OwnerExt {
    type Error;

    fn add_owner<O>(&mut self, owner: &Arc<O>) -> Result<(), Self::Error>
    where
        O: Resource<DynamicType = ()>;
}

#[derive(Debug, thiserror::Error)]
#[error("Could not take owner reference.")]
pub struct OwnerRefError;

impl OwnerExt for DynamicObject {
    type Error = OwnerRefError;

    fn add_owner<O>(&mut self, owner: &Arc<O>) -> Result<(), Self::Error>
    where
        O: Resource<DynamicType = ()>,
    {
        let owner_ref = owner.controller_owner_ref(&()).ok_or(OwnerRefError)?;
        self.owner_references_mut().push(owner_ref);

        Ok(())
    }
}

pub trait StatusExt<K> {
    type Status;
    type Error;

    async fn set_status<O>(&self, object: &O, status: Self::Status) -> Result<(), Self::Error>
    where
        O: Deref<Target = K>;
}

impl StatusExt<DHCPTemplate> for Api<DHCPTemplate> {
    type Status = DHCPTemplateStatus;
    type Error = ApiExtError;

    async fn set_status<O>(&self, object: &O, mut status: Self::Status) -> Result<(), Self::Error>
    where
        O: Deref<Target = DHCPTemplate>,
    {
        let name = object.name().ok_or(Self::Error::ResourceName)?;
        let params = PostParams::default();

        let mut current = self.get_status(&name).await?;

        status.conditions = status
            .conditions
            .into_iter()
            .chain(
                current
                    .status
                    .map(|status| status.conditions)
                    .unwrap_or_default()
                    .into_iter(),
            )
            .unique_by(|condition| condition.type_)
            .collect();

        current.status = Some(status);

        self.replace_status(&name, &params, &current).await?;
        Ok(())
    }
}
