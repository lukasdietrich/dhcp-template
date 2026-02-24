use std::fmt::{Debug, Display};

use kube::{Api, runtime::reflector::Lookup};
use tracing::{Level, instrument};

use crate::k8s::dynamic_ext::{
    apply_ext::{ApplyError, ApplyExt},
    delete_ext::{DeleteError, DeleteExt},
    managed_ext::{ManagedError, ManagedExt, Status},
};

#[derive(Debug, thiserror::Error)]
pub enum SafeError<E> {
    #[error(transparent)]
    Managed(#[from] ManagedError),

    #[error("Missing resource name.")]
    ResourceName,

    #[error("Will not execute api on foreign object.")]
    Foreign,

    #[error(transparent)]
    Safe(E),
}

pub trait SafeExt<K> {
    async fn safe_apply(&self, patch: &K) -> Result<K, SafeError<ApplyError>>;
    async fn safe_delete(&self, name: &str) -> Result<(), SafeError<DeleteError>>;
}

impl<K> SafeExt<K> for Api<K>
where
    K: Debug + Lookup,
    Self: ApplyExt<K> + DeleteExt<K> + ManagedExt<K>,
{
    #[instrument(skip(self), err(level = Level::WARN))]
    async fn safe_apply(&self, patch: &K) -> Result<K, SafeError<ApplyError>> {
        let name = patch.name().ok_or(SafeError::ResourceName)?;
        is_safe(self, &name).await?;

        self.apply_object(patch).await.map_err(SafeError::Safe)
    }

    #[instrument(skip(self), err(level = Level::WARN))]
    async fn safe_delete(&self, name: &str) -> Result<(), SafeError<DeleteError>> {
        is_safe(self, name).await?;
        self.delete_object(name).await.map_err(SafeError::Safe)
    }
}

#[instrument(skip(api), ret(level = Level::DEBUG), err(level = Level::WARN))]
async fn is_safe<A, K, E>(api: &A, name: &str) -> Result<(), SafeError<E>>
where
    A: ManagedExt<K>,
    E: Display,
{
    match api.is_managed(name).await? {
        Status::Managed | Status::Absent => Ok(()),
        Status::Foreign => Err(SafeError::Foreign),
    }
}
