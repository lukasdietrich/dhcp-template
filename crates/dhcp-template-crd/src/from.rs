use kube::{
    CustomResourceExt as _,
    api::{ApiResource, DynamicObject, TypeMeta},
};

use crate::{DHCPTemplate, ObjectRef};

#[derive(Debug, thiserror::Error)]
pub enum ObjectRefError {
    #[error("Cannot reference object: Missing types.")]
    MissingTypes,

    #[error("Cannot reference object: Missing name.")]
    MissingName,
}

impl TryFrom<&ObjectRef> for ObjectRef {
    type Error = ObjectRefError;

    fn try_from(object: &ObjectRef) -> Result<Self, Self::Error> {
        Ok(object.clone())
    }
}

impl TryFrom<&DynamicObject> for ObjectRef {
    type Error = ObjectRefError;

    fn try_from(object: &DynamicObject) -> Result<Self, Self::Error> {
        let TypeMeta { api_version, kind } =
            object.types.clone().ok_or(ObjectRefError::MissingTypes)?;

        let namespace = object.metadata.namespace.clone();
        let name = object
            .metadata
            .name
            .clone()
            .ok_or(ObjectRefError::MissingName)?;

        let object_ref = Self {
            api_version,
            kind,
            namespace,
            name,
        };

        Ok(object_ref)
    }
}

impl TryFrom<&DHCPTemplate> for ObjectRef {
    type Error = ObjectRefError;

    fn try_from(object: &DHCPTemplate) -> Result<Self, Self::Error> {
        let ApiResource {
            api_version, kind, ..
        } = DHCPTemplate::api_resource();

        let name = object
            .metadata
            .name
            .clone()
            .ok_or(ObjectRefError::MissingName)?;

        let object_ref = Self {
            api_version,
            kind,
            namespace: None,
            name,
        };

        Ok(object_ref)
    }
}
