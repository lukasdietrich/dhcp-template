use kube::api::{DynamicObject, TypeMeta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ObjectRef {
    pub api_version: String,
    pub kind: String,
    pub namespace: Option<String>,
    pub name: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ObjectRefError {
    #[error("Cannot reference object: Missing types.")]
    MissingTypes,

    #[error("Cannot reference object: Missing name.")]
    MissingName,
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
