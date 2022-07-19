use std::pin::Pin;
use futures::stream::{Stream, StreamExt};

use crate::util;

pub struct LineBuilder {
    sources: Vec<Pin<Box<dyn Stream<Item = String>>>>,
}

impl LineBuilder {
    pub fn new() -> Self {
        LineBuilder { sources: Vec::new() }
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
