use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::{Sample, Source};

pub trait DoneSignal {
    fn call(self);
}

impl<T> DoneSignal for T
where
    T: FnOnce() -> (),
{
    fn call(self) {
        (self)()
    }
}

/// When the inner source is empty this decrements an `AtomicUsize`.
#[derive(Debug, Clone)]
pub struct WhenDone<I, S> {
    input: I,
    signal: Option<S>,
}

impl DoneSignal for Arc<AtomicUsize> {
    fn call(self) {
        self.fetch_sub(1, Ordering::Relaxed);
    }
}
pub type Done<I> = WhenDone<I, Arc<AtomicUsize>>;

impl<I, S> WhenDone<I, S> {
    #[inline]
    pub fn new(input: I, signal: S) -> Self {
        Self {
            input,
            signal: Some(signal),
        }
    }

    /// Returns a reference to the inner source.
    #[inline]
    pub fn inner(&self) -> &I {
        &self.input
    }

    /// Returns a mutable reference to the inner source.
    #[inline]
    pub fn inner_mut(&mut self) -> &mut I {
        &mut self.input
    }

    /// Returns the inner source.
    #[inline]
    pub fn into_inner(self) -> I {
        self.input
    }
}

impl<I, S> Iterator for WhenDone<I, S>
where
    I: Source,
    I::Item: Sample,
    S: DoneSignal,
{
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<I::Item> {
        let next = self.input.next();
        if next.is_none() {
            if let Some(signal) = self.signal.take() {
                signal.call();
            }
        }
        next
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.input.size_hint()
    }
}

impl<I, S> Source for WhenDone<I, S>
where
    I: Source,
    I::Item: Sample,
    S: DoneSignal,
{
    #[inline]
    fn current_frame_len(&self) -> Option<usize> {
        self.input.current_frame_len()
    }

    #[inline]
    fn channels(&self) -> u16 {
        self.input.channels()
    }

    #[inline]
    fn sample_rate(&self) -> u32 {
        self.input.sample_rate()
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        self.input.total_duration()
    }
}
