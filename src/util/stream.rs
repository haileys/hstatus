use futures::future::{self, Either};
use futures::stream::{self, Stream, StreamExt};

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
