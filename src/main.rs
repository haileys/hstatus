mod source;
mod util;

use std::path::Path;
use std::pin::Pin;
use futures::stream::{Stream, StreamExt};

struct StatusLineBuilder {
    sources: Vec<Pin<Box<dyn Stream<Item = String>>>>,
}

impl StatusLineBuilder {
    pub fn new() -> Self {
        StatusLineBuilder { sources: Vec::new() }
    }

    pub fn segment(mut self, emoji: &'static str, source: impl Stream<Item = Option<String>> + 'static) -> Self {
        let source = source.map(move |suffix| {
            if let Some(suffix) = suffix {
                format!("{} {}", emoji, suffix)
            } else {
                String::new()
            }
        });

        self.sources.push(Box::pin(source) as Pin<Box<dyn Stream<Item = String>>>);

        self
    }

    pub fn build(self) -> Option<impl Stream<Item = String>> {
        self.sources.into_iter()
            .reduce(|left, right| {
                Box::pin(util::stream::combine(left, right)
                    .map(|(l, r)| l + "   " + &r))
            })
    }
}

#[tokio::main]
async fn main() {
    let status = StatusLineBuilder::new()
        .segment("🔋 ", source::battery::battery())
        .segment("📶 ", source::wifi::ssid(Path::new("/var/run/wpa_supplicant/wlp4s0")))
        .segment("🕒 ", source::clock::clock())
        .build();

    if let Some(status) = status {
        futures::pin_mut!(status);

        while let Some(status_line) = status.next().await {
            println!("{}", status_line);
        }
    }
}
