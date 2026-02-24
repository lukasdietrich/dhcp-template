use std::fmt::Debug;

use kube::{
    Api,
    api::{Patch, PatchParams},
    runtime::reflector::Lookup,
};
use serde::{Serialize, de::DeserializeOwned};
use tracing::{Level, instrument};

#[derive(Debug, thiserror::Error)]
pub enum ApplyError {
    #[error(transparent)]
    Kube(#[from] kube::Error),

    #[error("Missing resource name.")]
    ResourceName,
}

pub trait ApplyExt<K> {
    async fn apply_object(&self, patch: &K) -> Result<K, ApplyError>;
}

impl<K> ApplyExt<K> for Api<K>
where
    K: Clone + Debug + Serialize + DeserializeOwned + Lookup,
{
    #[instrument(skip(self), err(level = Level::WARN))]
    async fn apply_object(&self, patch: &K) -> Result<K, ApplyError> {
        let name = patch.name().ok_or(ApplyError::ResourceName)?;
        let params = PatchParams::apply("dhcp-template-operator");
        let patch = Patch::Apply(patch);

        Ok(self.patch::<_>(&name, &params, &patch).await?)
    }
}
