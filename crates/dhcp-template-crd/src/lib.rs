use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, CustomResource, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "k8s.lukasdietrich.com",
    version = "v1alpha1",
    kind = "DHCPTemplate",
    namespaced
)]
#[serde(rename_all = "camelCase")]
pub struct DHCPTemplateSpec {
    template: String,
}
