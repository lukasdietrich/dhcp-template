use dhcp_template_api::Node;
use dhcp_template_crd::DHCPTemplateSpec;
use kube::api::DynamicObject;
use minijinja::{Environment, context};
use serde::Deserialize;
use serde_yaml::Deserializer;

pub trait ManifestTemplate<D> {
    fn render(&self, data: D) -> anyhow::Result<Vec<DynamicObject>>;
}

impl ManifestTemplate<Vec<Node>> for DHCPTemplateSpec {
    fn render(&self, data: Vec<Node>) -> anyhow::Result<Vec<DynamicObject>> {
        let environment = Environment::new();
        let manifests = environment.render_str(&self.template, context!(nodes => data))?;

        let objects: Vec<Option<DynamicObject>> = Deserializer::from_str(&manifests)
            .map(Option::<DynamicObject>::deserialize)
            .collect::<Result<_, serde_yaml::Error>>()?;

        Ok(objects.into_iter().flatten().collect())
    }
}
