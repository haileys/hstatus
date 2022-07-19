use futures::future::{self, Either};
use futures::stream::{self, Stream, StreamExt};

pub fn combine<T, U>(left: impl Stream<Item = T>, right: impl Stream<Item = U>)
    -> impl Stream<Item = (T, U)>
    where T: Clone, U: Clone
{
    let mut left_value = None;
    let mut right_value = None;

    stream::select(left.map(Either::Left), right.map(Either::Right))
        .filter_map(move |item| {
            match item {
                Either::Left(val) => { left_value = Some(val); }
                Either::Right(val) => { right_value = Some(val); }
            }

            future::ready(left_value.clone().zip(right_value.clone()))
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
