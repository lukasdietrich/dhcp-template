use std::fmt::Debug;

use kube::{
    Api,
    api::{Patch, PatchParams},
    runtime::reflector::Lookup,
};
use serde::{Serialize, de::DeserializeOwned};

pub trait ApiExt<K> {
    async fn apply(&self, patch: &K) -> Result<K, kube::Error>;
}

impl<K> ApiExt<K> for Api<K>
where
    K: Clone + Serialize + DeserializeOwned + Debug,
    K: Lookup,
{
    async fn apply(&self, patch: &K) -> Result<K, kube::Error> {
        let name = patch.name().unwrap();
        let params = PatchParams::apply("dhcp-template-operator");
        let patch = Patch::Apply(patch);

        self.patch::<_>(&name, &params, &patch).await
    }
}
