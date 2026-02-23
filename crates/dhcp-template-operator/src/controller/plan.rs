use std::collections::BTreeSet;

use dhcp_template_crd::{DHCPTemplateStatus, ObjectRef};
use kube::api::DynamicObject;
use tracing::{Level, instrument};

#[derive(Debug)]
pub struct Plan {
    pub apply: BTreeSet<ObjectRef>,
    pub delete: BTreeSet<ObjectRef>,
}

impl Plan {
    #[instrument(
        skip_all,
        ret(level = Level::DEBUG),
        err(level = Level::WARN),
    )]
    pub fn diff<'a>(
        status: &'_ Option<DHCPTemplateStatus>,
        manifests: &'a [DynamicObject],
    ) -> Result<Plan, <ObjectRef as TryFrom<&'a DynamicObject>>::Error> {
        let pending = status
            .clone()
            .map(|status| status.objects)
            .unwrap_or_default();

        let apply = manifests
            .iter()
            .map(ObjectRef::try_from)
            .collect::<Result<BTreeSet<_>, _>>()?;

        let delete = pending.difference(&apply).cloned().collect();
        let plan = Self { apply, delete };

        Ok(plan)
    }

    pub fn all(&self) -> BTreeSet<ObjectRef> {
        self.apply.union(&self.delete).cloned().collect()
    }
}
