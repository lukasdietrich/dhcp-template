use std::{
    marker::PhantomData,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result};
use async_stream::stream;
use async_trait::async_trait;
use dhcp_template_api::Interface;
use dhcp_template_stream::boxed::{BoxStream, BoxStreamExt};
use log::debug;
use notify::{Event, EventKind, RecursiveMode, Watcher, recommended_watcher};
use tokio::sync::mpsc::channel;
use tokio_stream::{Stream, StreamExt, once};

use crate::provider::Provider;

#[async_trait]
pub trait InterfaceReader {
    async fn read_interfaces(path: &Path) -> Result<Vec<Interface>>;
}

pub struct NotifyProvider<R> {
    path: PathBuf,
    reader: PhantomData<R>,
}

impl<R> NotifyProvider<R> {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            reader: PhantomData,
        }
    }
}

impl<R> Provider for NotifyProvider<R>
where
    R: InterfaceReader + Sync + Send,
{
    fn interfaces<'a>(&'a self) -> BoxStream<'a, Result<Vec<Interface>>> {
        let initial = once(Ok(vec![]));

        let file_changes = watch_path(&self.path)
            .chunks_timeout(64, Duration::from_secs(5))
            .map(|events| events.into_iter().collect::<Result<Vec<_>>>());

        initial
            .chain(file_changes)
            .then(async |res| match res {
                Ok(_) => R::read_interfaces(&self.path).await,
                Err(err) => Err(err),
            })
            .boxed()
    }
}

fn watch_path(path: &Path) -> impl Stream<Item = Result<Event>> {
    stream! {
        let (tx, mut rx) = channel(64);

        let mut watcher = recommended_watcher(move |res| {
            tx.blocking_send(res)
                .expect("Could not send watcher event to channel!");
        })?;

        watcher
            .watch(path, RecursiveMode::NonRecursive)
            .with_context(|| format!("Could not watch path {:?}.", path))?;

        debug!("Watching {:?} for changes.", path);

        while let Some(res) = rx.recv().await {
            let event = res?;

            if is_releant_event(&event) {
                yield Ok(event);
            }
        }
    }
}

fn is_releant_event(event: &Event) -> bool {
    matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}
