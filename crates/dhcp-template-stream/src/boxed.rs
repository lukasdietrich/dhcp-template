use std::pin::Pin;

use tokio_stream::Stream;

pub type BoxStream<'a, T> = Pin<Box<dyn Stream<Item = T> + Send + 'a>>;

pub trait BoxStreamExt: Stream {
    fn boxed<'a>(self) -> BoxStream<'a, Self::Item>
    where
        Self: Sized + Send + 'a;
}

impl<T: Stream> BoxStreamExt for T {
    fn boxed<'a>(self) -> BoxStream<'a, Self::Item>
    where
        Self: Sized + Send + 'a,
    {
        Box::pin(self)
    }
}
