use std::str::FromStr as _;

use dhcp_template_crd::ObjectRef;
use kube::{
    Api, Client,
    api::DynamicObject,
    core::{GroupVersion, gvk::ParseGroupVersionError},
    discovery::{Scope, pinned_kind},
};

#[derive(Debug, thiserror::Error)]
pub enum DiscoverError {
    #[error("Could not parse GroupVersioKind: {0}")]
    ParseGroupVersionError(#[from] ParseGroupVersionError),

    #[error("Kubernetes Error: {0}")]
    KubeError(#[from] kube::Error),

    #[error("Cannot discover api for cluster scoped object, when the object has a namespace.")]
    ClusterScopeWithNamespace,

    #[error("Cannot discover api for namespace scoped object, when the object has no namespace.")]
    NamespaceScopeWithoutNamespace,
}

pub trait Discover
where
    Self: Sized,
{
    type Error;
    type Object;

    async fn discover(self, client: &Client) -> Result<Api<Self::Object>, Self::Error>;
}

impl<O> Discover for O
where
    O: TryInto<ObjectRef>,
    O::Error: std::fmt::Debug,
{
    type Error = DiscoverError;
    type Object = DynamicObject;

    async fn discover(self, client: &Client) -> Result<Api<Self::Object>, Self::Error> {
        let object_ref: ObjectRef = self.try_into().unwrap();
        let gvk = GroupVersion::from_str(&object_ref.api_version)?.with_kind(&object_ref.kind);

        let (resource, capabilities) = pinned_kind(client, &gvk).await?;

        let api = match capabilities.scope {
            Scope::Cluster => {
                if object_ref.namespace.is_some() {
                    return Err(Self::Error::ClusterScopeWithNamespace);
                }

                Api::all_with(client.clone(), &resource)
            }
            Scope::Namespaced => {
                let namespace = object_ref
                    .namespace
                    .ok_or(Self::Error::NamespaceScopeWithoutNamespace)?;

                Api::namespaced_with(client.clone(), &namespace, &resource)
            }
        };

        Ok(api)
    }
}
