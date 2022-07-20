use std::pin::Pin;
use futures::stream::{Stream, StreamExt};

use crate::util;

pub struct LineBuilder {
    sources: Vec<Pin<Box<dyn Stream<Item = Option<String>>>>>,
}

impl LineBuilder {
    pub fn new() -> Self {
        LineBuilder { sources: Vec::new() }
    }

    pub fn segment(mut self, emoji: &'static str, source: impl Stream<Item = Option<String>> + 'static) -> Self {
        let source = source.map(move |suffix| {
            suffix.map(|suffix| format!("{} {}", emoji, suffix))
        });

        self.sources.push(Box::pin(source) as Pin<Box<dyn Stream<Item = Option<String>>>>);

        self
    }

    pub fn build(self) -> impl Stream<Item = String> {
        util::stream::combine_all(self.sources)
            .map(|segments| segments.into_iter()
                .filter_map(|segment| segment.flatten())
                .collect::<Vec<_>>())
            .map(|segments| segments.join("   "))
    }
}
