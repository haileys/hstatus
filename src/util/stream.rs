use futures::future::{self, Future, Either};
use futures::stream::{self, Stream, StreamExt};
use futures::task::Poll;

pub fn combine<T, U>(left: impl Stream<Item = T>, right: impl Stream<Item = U>)
    -> impl Stream<Item = (Option<T>, Option<U>)>
    where T: Clone, U: Clone
{
    let mut left_value = None;
    let mut right_value = None;

    stream::select(left.map(Either::Left), right.map(Either::Right))
        .map(move |item| {
            match item {
                Either::Left(val) => { left_value = Some(val); }
                Either::Right(val) => { right_value = Some(val); }
            }

            (left_value.clone(), right_value.clone())
        })
}

pub fn combine_all<I, T>(streams: I) -> impl Stream<Item = Vec<Option<T>>>
    where I: IntoIterator,
          I::Item: Stream<Item = T> + Unpin,
          T: Clone + 'static,
{
    let streams = streams.into_iter()
        .enumerate()
        .map(|(idx, stream)|
            stream.map(move |item| (idx, item)))
        .collect::<Vec<_>>();

    let mut values = vec![None; streams.len()];

    stream::select_all(streams)
        .map(move |(idx, item)| {
            values[idx] = Some(item);
            values.clone()
        })
}

pub fn dedup<T>(stream: impl Stream<Item = T>) -> impl Stream<Item = T>
    where T: Eq + Clone
{
    let mut element = None;

    stream.filter_map(move |item| {
        let element_ref = &mut element;

        if element_ref.as_ref() == Some(&item) {
            return future::ready(None);
        }

        *element_ref = Some(item.clone());
        future::ready(Some(item))
    })
}

pub fn from_future<T>(fut: impl Future<Output = impl Stream<Item = T>> + 'static) -> impl Stream<Item = T> {
    let mut fut = Box::pin(fut);
    let mut stream_slot = None;

    stream::poll_fn(move |cx| {
        if stream_slot.is_none() {
            match fut.as_mut().poll(cx) {
                Poll::Ready(stream) => { stream_slot = Some(Box::pin(stream)); }
                Poll::Pending => { return Poll::Pending; }
            }
        }

        if let Some(stream) = stream_slot.as_mut() {
            stream.as_mut().poll_next(cx)
        } else {
            Poll::Pending
        }
    })
}

pub fn follow_latest<T>(stream: impl Stream<Item = impl Stream<Item = T>>) -> impl Stream<Item = T> {
    let mut outer = Box::pin(stream);
    let mut inner_slot = None;

    stream::poll_fn(move |cx| {
        match outer.as_mut().poll_next(cx) {
            Poll::Ready(Some(inner)) => { inner_slot = Some(Box::pin(inner)); }
            Poll::Ready(None) => { return Poll::Ready(None); }
            Poll::Pending => {}
        }

        if let Some(inner) = inner_slot.as_mut() {
            match inner.as_mut().poll_next(cx) {
                Poll::Ready(Some(item)) => { return Poll::Ready(Some(item)); }
                Poll::Ready(None) => { inner_slot = None; }
                Poll::Pending => {}
            }
        }

        Poll::Pending
    })
}

pub fn flatten_result_stream<T, E>(stream: Result<impl Stream<Item = Result<T, E>>, E>) -> impl Stream<Item = Result<T, E>> {
    match stream {
        Err(e) => Either::Left(stream::once(future::ready(Err(e)))),
        Ok(s) => Either::Right(s),
    }
}
