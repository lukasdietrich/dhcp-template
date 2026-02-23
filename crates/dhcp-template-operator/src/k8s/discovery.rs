use std::{fmt::Debug, str::FromStr as _};

use dhcp_template_crd::{ObjectRef, ObjectRefError};
use kube::{
    Api, Client,
    api::DynamicObject,
    core::{GroupVersion, gvk::ParseGroupVersionError},
    discovery::{Scope, pinned_kind},
};
use tracing::{Level, instrument};

#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error(transparent)]
    ParseGroupVersion(#[from] ParseGroupVersionError),

    #[error(transparent)]
    Kube(#[from] kube::Error),

    #[error(transparent)]
    ObjectRef(#[from] ObjectRefError),

    #[error("Unexpected resource scope {0:?}.")]
    MismatchingScope(Scope),
}

pub trait Discover {
    async fn discover(&self, client: Client) -> Result<Api<DynamicObject>, DiscoveryError>;
}

impl Discover for DynamicObject {
    async fn discover(&self, client: Client) -> Result<Api<DynamicObject>, DiscoveryError> {
        let object_ref = ObjectRef::try_from(self)?;
        object_ref.discover(client).await
    }
}

impl Discover for ObjectRef {
    #[instrument(skip(client), err(level = Level::WARN))]
    async fn discover(&self, client: Client) -> Result<Api<DynamicObject>, DiscoveryError> {
        let group_version_kind = GroupVersion::from_str(&self.api_version)?.with_kind(&self.kind);
        let (resource, capabilities) = pinned_kind(&client, &group_version_kind).await?;

        let api = match capabilities.scope {
            Scope::Cluster => {
                if self.namespace.is_some() {
                    Err(DiscoveryError::MismatchingScope(Scope::Cluster))?;
                }

                Api::all_with(client, &resource)
            }
            Scope::Namespaced => {
                let namespace = self
                    .namespace
                    .as_ref()
                    .ok_or(DiscoveryError::MismatchingScope(Scope::Namespaced))?;

                Api::namespaced_with(client, namespace, &resource)
            }
        };

        Ok(api)
    }
}
