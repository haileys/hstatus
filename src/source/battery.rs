use futures::{Stream, StreamExt};
use std::path::Path;

use crate::util::file_contents;
use crate::util::stream::{combine, dedup};

pub fn battery() -> impl Stream<Item = Option<String>> {
    let charge_now = stream(Path::new("/sys/class/power_supply/BAT0/charge_now"));
    let charge_full = stream(Path::new("/sys/class/power_supply/BAT0/charge_full"));

    let stream = combine(charge_now, charge_full)
        .map(|(now, full)| {
            now.zip(full).map(|(now, full)| {
                let percent = now * 100 / full;
                format!("{}%", percent)
            })
        });

    dedup(stream)
}

fn stream(path: &Path) -> impl Stream<Item = Option<i32>> {
    file_contents::strings(path)
        .map(|string| string?.trim().parse().ok())
}
