use std::fmt::Debug;

use kube::{Resource, ResourceExt as _, api::DynamicObject};
use tracing::{Level, instrument};

use crate::k8s::dynamic_ext::labels::{MANAGED_BY_KEY, MANAGED_BY_VALUE};

#[derive(Debug, thiserror::Error)]
pub enum OwnerError {
    #[error("Could not take owner reference.")]
    OwnerRef,
}

pub trait OwnerExt {
    fn add_owner<O>(&mut self, owner: &O) -> Result<(), OwnerError>
    where
        O: Resource<DynamicType = ()> + Debug;
}

impl OwnerExt for DynamicObject {
    #[instrument(err(level = Level::WARN))]
    fn add_owner<O>(&mut self, owner: &O) -> Result<(), OwnerError>
    where
        O: Resource<DynamicType = ()> + Debug,
    {
        let owner_ref = owner
            .controller_owner_ref(&())
            .ok_or(OwnerError::OwnerRef)?;

        self.owner_references_mut().push(owner_ref);
        self.labels_mut()
            .insert(MANAGED_BY_KEY.into(), MANAGED_BY_VALUE.into());

        Ok(())
    }
}
