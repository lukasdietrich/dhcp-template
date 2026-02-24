use std::fmt::Debug;

use kube::{Api, ResourceExt, api::PartialObjectMeta};
use serde::de::DeserializeOwned;

use crate::k8s::dynamic_ext::labels::{MANAGED_BY_KEY, MANAGED_BY_VALUE};

pub trait ManagedObject {
    fn managed_by(&self) -> Option<&str>;
}

impl<R> ManagedObject for PartialObjectMeta<R>
where
    R: ResourceExt,
{
    fn managed_by(&self) -> Option<&str> {
        self.metadata
            .labels
            .as_ref()
            .and_then(|labels| labels.get(MANAGED_BY_KEY).map(String::as_str))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ManagedError {
    #[error(transparent)]
    Kube(#[from] kube::Error),
}

pub enum Status {
    Managed,
    Foreign,
    Absent,
}

pub trait ManagedExt<K> {
    async fn is_managed(&self, name: &str) -> Result<Status, ManagedError>;
}

impl<K> ManagedExt<K> for Api<K>
where
    K: Clone + Debug + DeserializeOwned,
    PartialObjectMeta<K>: ManagedObject,
{
    async fn is_managed(&self, name: &str) -> Result<Status, ManagedError> {
        let managed = match self.get_metadata_opt(name).await? {
            Some(object) if matches!(object.managed_by(), Some(MANAGED_BY_VALUE)) => {
                Status::Managed
            }
            Some(_) => Status::Foreign,
            None => Status::Absent,
        };

        Ok(managed)
    }
}
