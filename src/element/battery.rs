use futures::{Stream, StreamExt, future};
use std::path::Path;

use crate::util::file_contents;
use crate::util::stream::combine;

pub fn battery() -> impl Stream<Item = String> {
    let charge_now = file_contents::strings(&Path::new("/sys/class/power_supply/BAT0/charge_now"));
    let charge_full = file_contents::strings(&Path::new("/sys/class/power_supply/BAT0/charge_full"));

    combine(charge_now, charge_full)
        .filter_map(|(now, full)| {
            let now = now.trim().parse::<i32>().ok();
            let full = full.trim().parse::<i32>().ok();

            future::ready(
                now.zip(full).map(|(now, full)| {
                    let percent = now * 100 / full;
                    format!("{}%", percent)
                })
            )
        })
}
