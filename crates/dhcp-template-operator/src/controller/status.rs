use std::{collections::BTreeSet, ops::Deref};

use dhcp_template_crd::{Condition, DHCPTemplate, DHCPTemplateStatus, ObjectRef, Reason, Type};
use kube::Api;

use crate::k8s::api_ext::StatusExt;

pub trait DHCPTemplateStatusExt
where
    Self: StatusExt<DHCPTemplate>,
{
    async fn set_status_pending<O>(
        &self,
        object: &O,
        objects: BTreeSet<ObjectRef>,
    ) -> Result<(), Self::Error>
    where
        O: Deref<Target = DHCPTemplate>;

    async fn set_status_ready<O>(
        &self,
        object: &O,
        objects: BTreeSet<ObjectRef>,
    ) -> Result<(), Self::Error>
    where
        O: Deref<Target = DHCPTemplate>;
}

impl DHCPTemplateStatusExt for Api<DHCPTemplate> {
    async fn set_status_pending<O>(
        &self,
        object: &O,
        objects: BTreeSet<ObjectRef>,
    ) -> Result<(), Self::Error>
    where
        O: Deref<Target = DHCPTemplate>,
    {
        self.set_status(
            object,
            DHCPTemplateStatus {
                objects,
                conditions: vec![Condition::new(
                    object,
                    Reason::Reconciliation,
                    Type::Pending,
                    "Beginning to reconcile rendered objects.".to_owned(),
                )],
            },
        )
        .await
    }

    async fn set_status_ready<O>(
        &self,
        object: &O,
        objects: BTreeSet<ObjectRef>,
    ) -> Result<(), Self::Error>
    where
        O: Deref<Target = DHCPTemplate>,
    {
        self.set_status(
            object,
            DHCPTemplateStatus {
                objects,
                conditions: vec![Condition::new(
                    object,
                    Reason::AllObjectsReady,
                    Type::Ready,
                    "Applied objects successfully.".to_owned(),
                )],
            },
        )
        .await
    }
}
