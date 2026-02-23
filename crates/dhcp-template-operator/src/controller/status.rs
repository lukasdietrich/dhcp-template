use std::collections::BTreeSet;

use dhcp_template_crd::{Condition, DHCPTemplate, DHCPTemplateStatus, ObjectRef, Reason, Type};
use kube::Api;
use tracing::{Level, instrument};

use crate::k8s::api_ext::StatusExt;

pub trait DHCPTemplateStatusExt
where
    Self: StatusExt<DHCPTemplate>,
{
    async fn set_template_status(
        &self,
        object: &DHCPTemplate,
        objects: BTreeSet<ObjectRef>,
        reason: Reason,
        type_: Type,
        message: String,
    ) -> Result<(), Self::Error>;

    async fn set_template_error(
        &self,
        object: &DHCPTemplate,
        reason: Reason,
        message: String,
    ) -> Result<(), Self::Error>;
}

impl DHCPTemplateStatusExt for Api<DHCPTemplate> {
    #[instrument(
        skip(self, object, objects, message),
        ret(level = Level::DEBUG),
        err(level = Level::ERROR),
    )]
    async fn set_template_status(
        &self,
        object: &DHCPTemplate,
        objects: BTreeSet<ObjectRef>,
        reason: Reason,
        type_: Type,
        message: String,
    ) -> Result<(), Self::Error> {
        self.set_status(
            object,
            DHCPTemplateStatus {
                objects,
                conditions: vec![Condition::new(object, reason, type_, message)],
            },
        )
        .await
    }

    async fn set_template_error(
        &self,
        object: &DHCPTemplate,
        reason: Reason,
        message: String,
    ) -> Result<(), Self::Error> {
        self.set_template_status(
            object,
            object
                .status
                .as_ref()
                .map(|status| status.objects.clone())
                .unwrap_or_default(),
            reason,
            Type::Error,
            message,
        )
        .await
    }
}
