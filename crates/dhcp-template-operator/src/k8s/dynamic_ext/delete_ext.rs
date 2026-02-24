use std::fmt::Debug;

use kube::{Api, Error, api::DeleteParams};
use serde::de::DeserializeOwned;
use tracing::{Level, debug, instrument};

#[derive(Debug, thiserror::Error)]
pub enum DeleteError {
    #[error(transparent)]
    Kube(#[from] kube::Error),
}

pub trait DeleteExt<K> {
    async fn delete_object(&self, name: &str) -> Result<(), DeleteError>;
}

impl<K> DeleteExt<K> for Api<K>
where
    K: Clone + Debug + DeserializeOwned,
{
    #[instrument(skip(self), err(level = Level::WARN))]
    async fn delete_object(&self, name: &str) -> Result<(), DeleteError> {
        let params = DeleteParams::default();

        match self.delete(name, &params).await {
            Err(Error::Api(status)) if status.is_not_found() => debug!("Object already absent."),
            result => {
                let _ = result?;
            }
        }

        Ok(())
    }
}
