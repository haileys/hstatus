use std::time::Duration;

use chrono::Local;
use futures::{Stream, stream};

pub fn clock() -> impl Stream<Item = Option<String>> {
    stream::unfold(Duration::from_secs(0), |delay| async move {
        tokio::time::sleep(delay).await;

        let time = Local::now();
        let formatted = time.format("%d %a %H:%M:%S").to_string();

        let delay = Duration::from_micros((1_000_001 - time.timestamp_subsec_micros()) as u64);

        Some((Some(formatted), delay))
    })
}
