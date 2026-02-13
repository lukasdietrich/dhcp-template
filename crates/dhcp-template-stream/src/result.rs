use tokio_stream::{Stream, StreamExt, adapters::Map};

pub trait ResultStreamExt<T, E>
where
    Self: Stream<Item = Result<T, E>> + StreamExt,
{
    fn try_map<R, F>(self, f: F) -> Map<Self, impl FnMut(Self::Item) -> Result<R, E>>
    where
        Self: Sized,
        F: FnMut(T) -> R;
}

impl<T, E, S> ResultStreamExt<T, E> for S
where
    Self: Stream<Item = Result<T, E>> + StreamExt,
{
    fn try_map<R, F>(self, mut f: F) -> Map<Self, impl FnMut(Self::Item) -> Result<R, E>>
    where
        F: FnMut(T) -> R,
    {
        self.map(move |res| res.map(&mut f))
    }
}
