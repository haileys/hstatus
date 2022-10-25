use futures::{Stream, StreamExt};
use futures::future::Either;
use std::io;
use std::fs;
use std::path::{Path, PathBuf};

use crate::util::file_contents;
use crate::util::stream::{combine, dedup};

pub fn auto() -> impl Stream<Item = Option<String>> {
    fn battery_path() -> Result<Option<PathBuf>, io::Error> {
        Ok(fs::read_dir("/sys/class/power_supply")?
            .filter_map(|ent| ent.ok())
            .filter(|ent| ent.file_name().to_string_lossy().starts_with("BAT"))
            .map(|ent| ent.path())
            .nth(0))
    }

    battery_path()
        .unwrap_or_default()
        .map(|path| battery(&path))
        .map(Either::Left)
        .unwrap_or(Either::Right(futures::stream::empty()))
}

pub fn battery(path: &Path) -> impl Stream<Item = Option<String>> {
    let charge_now = stream(&path.join("charge_now"));
    let charge_full = stream(&path.join("charge_full"));

    dedup(
        combine(charge_now, charge_full)
            .map(|(now, full)| {
                now.flatten().zip(full.flatten()).map(|(now, full)| {
                    let percent = now * 100 / full;
                    format!("{}%", percent)
                })
            }))
}

fn stream(path: &Path) -> impl Stream<Item = Option<i32>> {
    file_contents::strings(path)
        .map(|string| string?.trim().parse().ok())
}
