use futures::{Stream, StreamExt};
use std::path::Path;

use crate::util::file_contents;
use crate::util::stream::{combine, dedup};

pub fn battery() -> impl Stream<Item = Option<String>> {
    let charge_now = file_contents::strings(&Path::new("/sys/class/power_supply/BAT0/charge_now"));
    let charge_full = file_contents::strings(&Path::new("/sys/class/power_supply/BAT0/charge_full"));

    let stream = combine(charge_now, charge_full)
        .map(|(now, full)| {
            fn parse(opt: Option<String>) -> Option<i32> {
                opt.and_then(|val| val.trim().parse().ok())
            }

            let now = parse(now);
            let full = parse(full);

            now.zip(full).map(|(now, full)| {
                let percent = now * 100 / full;
                format!("{}%", percent)
            })
        });

    dedup(stream)
}
