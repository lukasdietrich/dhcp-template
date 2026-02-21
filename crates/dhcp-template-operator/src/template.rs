use std::sync::Arc;

use dhcp_template_api::Node;
use dhcp_template_crd::DHCPTemplateSpec;
use kube::api::DynamicObject;
use minijinja::{Environment, context};
use serde::Deserialize;
use serde_yaml::Deserializer;
use tracing::{Level, instrument};

#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    #[error("Could not render template: {0}")]
    Render(#[from] minijinja::Error),

    #[error("Could not parse rendered manifests: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

pub trait ManifestTemplate<D> {
    fn render(&self, data: D) -> Result<Vec<DynamicObject>, TemplateError>;
}

impl ManifestTemplate<Vec<Arc<Node>>> for DHCPTemplateSpec {
    #[instrument(skip_all, ret(level = Level::DEBUG), err(level = Level::WARN))]
    fn render(&self, data: Vec<Arc<Node>>) -> Result<Vec<DynamicObject>, TemplateError> {
        let environment = Environment::new();
        let manifests = environment.render_str(&self.template, context!(nodes => data))?;

        let objects: Vec<Option<DynamicObject>> = Deserializer::from_str(&manifests)
            .map(Option::<DynamicObject>::deserialize)
            .collect::<Result<_, serde_yaml::Error>>()?;

        Ok(objects.into_iter().flatten().collect())
    }
}
