mod from;

use std::collections::BTreeSet;

pub use from::ObjectRefError;
use k8s_openapi::{apimachinery::pkg::apis::meta::v1::Time, jiff::Timestamp};
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, CustomResource, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "k8s.lukasdietrich.com",
    version = "v1alpha1",
    kind = "DHCPTemplate",
    status = DHCPTemplateStatus,
    printcolumn = r#"{"name":"Ready", "type":"string", "jsonPath":".status.conditions[0].type"}"#,
    printcolumn = r#"{"name":"Message", "type":"string", "jsonPath":".status.conditions[0].message"}"#,
)]
#[serde(rename_all = "camelCase")]
pub struct DHCPTemplateSpec {
    pub template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DHCPTemplateStatus {
    pub objects: BTreeSet<ObjectRef>,
    pub conditions: Vec<Condition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Condition {
    pub last_transition_time: Time,
    pub observed_generation: Option<i64>,
    pub status: Status,
    #[serde(rename = "type")]
    pub type_: Type,
    pub reason: Reason,
    pub message: String,
}

impl Condition {
    #[must_use]
    pub fn new(object: &DHCPTemplate, reason: Reason, type_: Type, message: String) -> Self {
        Self {
            last_transition_time: Time::from(Timestamp::now()),
            observed_generation: object.metadata.generation,
            status: Status::True,
            type_,
            reason,
            message,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum Reason {
    Reconciliation,
    TemplateEvaluation,
    PlanningObjects,
    AllObjectsReady,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum Status {
    True,
    False,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum Type {
    Pending,
    Ready,
    Error,
    #[serde(other)]
    Unknown,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub struct ObjectRef {
    pub api_version: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    pub name: String,
}
