use std::{fmt::Debug, str::FromStr as _};

use dhcp_template_crd::{ObjectRef, ObjectRefError};
use kube::{
    Api, Client,
    api::DynamicObject,
    core::{GroupVersion, gvk::ParseGroupVersionError},
    discovery::{Scope, pinned_kind},
};

#[derive(Debug, thiserror::Error)]
pub enum DiscoverError {
    #[error(transparent)]
    ParseGroupVersionError(#[from] ParseGroupVersionError),

    #[error(transparent)]
    KubeError(#[from] kube::Error),

    #[error("Cannot discover api for cluster scoped object, when the object has a namespace.")]
    ClusterScopeWithNamespace,

    #[error("Cannot discover api for namespace scoped object, when the object has no namespace.")]
    NamespaceScopeWithoutNamespace,

    #[error(transparent)]
    ObjectRefError(#[from] ObjectRefError),
}

pub trait Discover<'a>
where
    Self: Sized,
{
    type Error;
    type Object;

    async fn discover(&'a self, client: Client) -> Result<Api<Self::Object>, Self::Error>;
}

impl<'a, O> Discover<'a> for O
where
    O: 'a,
    ObjectRef: TryFrom<&'a O>,
    <ObjectRef as TryFrom<&'a O>>::Error: Debug,
    DiscoverError: From<<ObjectRef as TryFrom<&'a O>>::Error>,
{
    type Error = DiscoverError;
    type Object = DynamicObject;

    async fn discover(&'a self, client: Client) -> Result<Api<Self::Object>, Self::Error> {
        let object_ref: ObjectRef = self.try_into()?;
        let gvk = GroupVersion::from_str(&object_ref.api_version)?.with_kind(&object_ref.kind);

        let (resource, capabilities) = pinned_kind(&client, &gvk).await?;

        let api = match capabilities.scope {
            Scope::Cluster => {
                if object_ref.namespace.is_some() {
                    return Err(Self::Error::ClusterScopeWithNamespace);
                }

                Api::all_with(client, &resource)
            }
            Scope::Namespaced => {
                let namespace = object_ref
                    .namespace
                    .ok_or(Self::Error::NamespaceScopeWithoutNamespace)?;

                Api::namespaced_with(client, &namespace, &resource)
            }
        };

        Ok(api)
    }
}
