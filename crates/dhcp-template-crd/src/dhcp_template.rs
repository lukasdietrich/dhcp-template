use k8s_openapi::apimachinery::pkg::apis::meta::v1::Time;
use kube::{CustomResource, CustomResourceExt as _, api::ApiResource};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{ObjectRef, ObjectRefError};

#[derive(Debug, Default, Clone, CustomResource, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "k8s.lukasdietrich.com",
    version = "v1alpha1",
    kind = "DHCPTemplate",
    status = DHCPTemplateStatus
)]
#[serde(rename_all = "camelCase")]
pub struct DHCPTemplateSpec {
    pub template: String,
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DHCPTemplateStatus {
    pub objects: Vec<ObjectRef>,
    pub conditions: Vec<Condition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Condition {
    pub last_transition_time: Time,
    pub message: String,
    pub observed_generation: Option<i64>,
    pub reason: Reason,
    pub status: Status,
    #[serde(rename = "type")]
    pub type_: Type,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum Reason {
    ConfigurationChange,
    NodeChange,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum Status {
    True,
    False,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum Type {
    Pending,
    Ready,
    Error,
}
