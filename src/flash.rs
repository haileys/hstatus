use std::time::Duration;
use std::path::Path;
use std::sync::Arc;

use futures::{Stream, StreamExt};
use tokio::io::{self, AsyncBufRead, AsyncBufReadExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::watch;
use tokio_stream::wrappers::{WatchStream, UnixListenerStream, LinesStream};

use crate::util::future::defer;

const FLASH_DURATION: Duration = Duration::from_secs(1);

pub fn bind(path: &Path) -> Result<impl Stream<Item = Option<String>>, io::Error> {
    // delete socket path before trying to bind it
    let _ = std::fs::remove_file(path);

    let listener = UnixListener::bind(path)?;

    let (tx, rx) = watch::channel(());

    let lines = UnixListenerStream::new(listener)
        .filter_map(|socket| async { socket.ok() })
        .map(|socket| BufReader::new(socket))
        .flat_map_unordered(None, |reader| LinesStream::new(reader.lines()))
        .filter_map(|line| async { line.ok() })
        .inspect({
            let tx = Arc::new(tx);
            let mut cancel = None;

            move |line| {
                let tx = tx.clone();
                cancel = Some(defer(FLASH_DURATION, async move {
                    let _ = tx.send(());
                }));
            }
        });

    let lines = lines.map(Some);
    let clears = WatchStream::new(rx).map(|()| None);

    Ok(futures::stream::select(lines, clears))
}
