use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use async_stream::stream;
use async_trait::async_trait;
use dhcp_template_api::Interface;
use futures_time::{stream::StreamExt as _, time::Duration};
use futures_util::{
    Stream, StreamExt as _, TryStreamExt,
    future::{Ready, ready},
    stream::{BoxStream, once},
};
use log::debug;
use notify::{Event, EventKind, RecursiveMode, Watcher, recommended_watcher};
use tokio::sync::mpsc::channel;

use crate::provider::Provider;

#[async_trait]
pub trait InterfaceReader {
    async fn interfaces(&self, path: &Path) -> Result<Vec<Interface>>;
}

pub struct NotifyProvider<R> {
    path: PathBuf,
    reader: R,
}

impl<R> NotifyProvider<R> {
    pub fn new(path: PathBuf, reader: R) -> Self {
        Self { path, reader }
    }
}

impl<R> Provider for NotifyProvider<R>
where
    R: InterfaceReader + Sync + Send,
{
    fn interfaces<'a>(&'a self) -> BoxStream<'a, Result<Vec<Interface>>> {
        let initial = once(async { Ok(Event::new(EventKind::Other)) });
        let changes = watch_path(&self.path)
            .try_filter(is_relevant_event)
            .into_stream()
            .debounce(Duration::from_secs(10))
            .inspect_ok(|_| debug!("Change on filesystem detected, reloading interfaces."));

        initial
            .chain(changes)
            .and_then(async |_| self.reader.interfaces(&self.path).await)
            .boxed()
    }
}

fn is_relevant_event(event: &Event) -> Ready<bool> {
    ready(matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    ))
}

fn watch_path(path: &Path) -> impl Stream<Item = Result<Event>> {
    let (tx, mut rx) = channel(64);

    stream! {
        let mut watcher = recommended_watcher(move |res| {
            tx.blocking_send(res)
                .expect("Could not send watcher event to channel!");
        })?;

        watcher
            .watch(path, RecursiveMode::NonRecursive)
            .with_context(|| format!("Could not watch path {:?}.", path))?;

        debug!("Watching {:?} for changes.", path);

        while let Some(res) = rx.recv().await {
            yield Ok(res?);
        }
    }
}
