mod source;
mod status;
mod util;

use std::path::Path;
use futures::stream::StreamExt;

#[tokio::main]
async fn main() {
    let status = status::LineBuilder::new()
        .segment("🔋 ", source::battery::battery())
        .segment("📶 ", source::wifi::ssid(Path::new("/var/run/wpa_supplicant/wlp4s0")))
        .segment("🕒 ", source::clock::clock())
        .build()
        .expect("could not build status line");

    futures::pin_mut!(status);

    while let Some(status_line) = status.next().await {
        println!("{}", status_line);
    }
}
