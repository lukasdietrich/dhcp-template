mod context;
mod plan;
mod reconcile;
mod status;

use std::{sync::Arc, time::Duration};

use anyhow::Result;
use dhcp_template_crd::DHCPTemplate;
use futures_util::StreamExt as _;
use kube::{
    Api, Client,
    runtime::{
        Controller, PredicateConfig, WatchStreamExt as _, controller::Config, predicates,
        reflector, watcher,
    },
};
use tracing::warn;

use crate::{
    controller::{
        context::Context,
        reconcile::{error_policy, reconcile},
    },
    state::State,
};

pub async fn run(state: State) -> Result<()> {
    let client = Client::try_default().await?;

    let api: Api<DHCPTemplate> = Api::all(client.clone());
    let state_changes = state.changes();

    let (reader, writer) = reflector::store();
    let stream = watcher(api, watcher::Config::default())
        .default_backoff()
        .reflect(writer)
        .applied_objects()
        .predicate_filter(predicates::generation, PredicateConfig::default());

    let ctx = Arc::new(Context::from((client, state)));

    Controller::for_stream(stream, reader)
        .with_config(
            Config::default()
                .concurrency(1)
                .debounce(Duration::from_secs(10)),
        )
        .reconcile_all_on(state_changes)
        .run(reconcile, error_policy, ctx)
        .for_each(|res| async move {
            if let Err(error) = res {
                warn!("Reconciliation failed: {}", error);
            }
        })
        .await;

    Ok(())
}
