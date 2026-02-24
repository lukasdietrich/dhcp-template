use std::collections::BTreeSet;

use dhcp_template_crd::{Condition, DHCPTemplate, DHCPTemplateStatus, ObjectRef, Reason, Type};
use kube::Api;
use tracing::{Level, instrument};

use crate::k8s::template_ext::status_ext::{StatusError, StatusExt};

pub trait DHCPTemplateStatusConditionExt
where
    Self: StatusExt<DHCPTemplate>,
{
    async fn add_condition_with_objects(
        &self,
        object: &DHCPTemplate,
        objects: BTreeSet<ObjectRef>,
        reason: Reason,
        type_: Type,
        message: String,
    ) -> Result<(), StatusError>;

    async fn add_condition(
        &self,
        object: &DHCPTemplate,
        reason: Reason,
        type_: Type,
        message: String,
    ) -> Result<(), StatusError>;
}

impl DHCPTemplateStatusConditionExt for Api<DHCPTemplate> {
    #[instrument(
        skip(self, object, objects, message),
        ret(level = Level::DEBUG),
        err(level = Level::ERROR),
    )]
    async fn add_condition_with_objects(
        &self,
        object: &DHCPTemplate,
        objects: BTreeSet<ObjectRef>,
        reason: Reason,
        type_: Type,
        message: String,
    ) -> Result<(), StatusError> {
        self.set_status(
            object,
            DHCPTemplateStatus {
                objects,
                conditions: vec![Condition::new(object, reason, type_, message)],
            },
        )
        .await
    }
    async fn add_condition(
        &self,
        object: &DHCPTemplate,
        reason: Reason,
        type_: Type,
        message: String,
    ) -> Result<(), StatusError> {
        self.add_condition_with_objects(
            object,
            object
                .status
                .as_ref()
                .map(|status| status.objects.clone())
                .unwrap_or_default(),
            reason,
            type_,
            message,
        )
        .await
    }
}
