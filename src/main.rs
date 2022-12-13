mod flash;
mod source;
mod status;
mod util;

use std::path::{Path, PathBuf};
use futures::future::Either;
use futures::stream::{self, Stream, StreamExt};
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opt {
    #[structopt(short, long)]
    socket: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    let opt = Opt::from_args();

    let status = status::LineBuilder::new()
        .segment("🔋 ", source::battery::auto())
        .segment("📶 ", source::wifi::networkmanager::network())
        .segment("🕒 ", source::clock::clock())
        .build();

    let flash = flash(opt.socket.as_deref());

    let display = merge_flash(flash, status);

    futures::pin_mut!(display);

    while let Some(line) = display.next().await {
        println!("{}", line);
    }
}

fn merge_flash(
    flash: impl Stream<Item = Option<String>>,
    stream: impl Stream<Item = String>,
) -> impl Stream<Item = String> {
    util::stream::combine(flash, stream)
        .map(|(flash, line)| flash.flatten().or(line).unwrap_or_default())
}

fn flash(path: Option<&Path>) -> impl Stream<Item = Option<String>> {
    match path {
        Some(path) => Either::Left(flash::bind(path).expect("flash::bind")),
        None => Either::Right(stream::empty()),
    }
}
