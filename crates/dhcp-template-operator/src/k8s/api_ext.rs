use std::fmt::Debug;

use dhcp_template_crd::{DHCPTemplate, DHCPTemplateStatus};
use itertools::Itertools as _;
use kube::{Api, api::PostParams, runtime::reflector::Lookup};

#[derive(Debug, thiserror::Error)]
pub enum ApiExtError {
    #[error(transparent)]
    KubeError(#[from] kube::Error),

    #[error("Missing resource name.")]
    ResourceName,
}

pub trait StatusExt<K> {
    type Status;
    type Error;

    async fn set_status(&self, object: &K, status: Self::Status) -> Result<(), Self::Error>;
}

impl StatusExt<DHCPTemplate> for Api<DHCPTemplate> {
    type Status = DHCPTemplateStatus;
    type Error = ApiExtError;

    async fn set_status(
        &self,
        object: &DHCPTemplate,
        mut status: Self::Status,
    ) -> Result<(), Self::Error> {
        let name = object.name().ok_or(Self::Error::ResourceName)?;
        let params = PostParams::default();

        let mut current = self.get_status(&name).await?;

        status.conditions = status
            .conditions
            .into_iter()
            .chain(
                current
                    .status
                    .map(|status| status.conditions)
                    .unwrap_or_default()
                    .into_iter(),
            )
            .unique_by(|condition| condition.type_)
            .collect();

        current.status = Some(status);

        self.replace_status(&name, &params, &current).await?;
        Ok(())
    }
}
