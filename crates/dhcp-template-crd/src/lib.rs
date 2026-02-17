use chrono::{DateTime, Utc};
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, CustomResource, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "k8s.lukasdietrich.com",
    version = "v1alpha1",
    kind = "DHCPTemplate",
    namespaced,
    status = DHCPTemplateStatus
)]
#[serde(rename_all = "camelCase")]
pub struct DHCPTemplateSpec {
    pub template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DHCPTemplateStatus {
    pub last_updated: DateTime<Utc>,
    pub cause: Cause,
    pub state: State,
    pub message: Option<String>,
    pub owned_resources: Vec<Resource>,
    pub pending_resources: Vec<Resource>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum Cause {
    ConfigurationChanged,
    NodesChanges,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum State {
    Reconciled,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Resource {
    pub kind: String,
    pub namespace: Option<String>,
    pub name: String,
}
