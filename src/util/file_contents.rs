use futures::{Stream, StreamExt};
use std::path::Path;
use std::time::Duration;

use tokio_stream::wrappers::IntervalStream;

pub fn strings(path: &Path) -> impl Stream<Item = Option<String>> {
    let path = path.to_owned();

    // TODO - use inotify or something
    let interval = tokio::time::interval(Duration::from_secs(1));

    IntervalStream::new(interval)
        .then(move |_| tokio::fs::read_to_string(path.clone()))
        .map(|s| Result::ok(s))
}
