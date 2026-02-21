use std::{fmt::Debug, sync::Arc};

use kube::{
    Api, Resource, ResourceExt as _,
    api::{DynamicObject, Patch, PatchParams},
    runtime::reflector::Lookup,
};
use serde::{Serialize, de::DeserializeOwned};

#[derive(Debug, thiserror::Error)]
pub enum ApplyError {
    #[error(transparent)]
    KubeError(#[from] kube::Error),

    #[error("Missing resource name.")]
    ResourceName,
}

pub trait ApiExt<K> {
    async fn apply(&self, patch: &K) -> Result<K, ApplyError>;
}

impl<K> ApiExt<K> for Api<K>
where
    K: Clone + Debug + Serialize + DeserializeOwned + Lookup,
{
    async fn apply(&self, patch: &K) -> Result<K, ApplyError> {
        let name = patch.name().ok_or(ApplyError::ResourceName)?;
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
