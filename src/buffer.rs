use std::iter::FromIterator;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

use crate::source::SourceDuration;
use crate::{Sample, Source};

pub type SamplesBuffer<S> = GenericBuffer<S, Vec<S>>;
pub type StaticSamplesBuffer<S> = GenericBuffer<S, &'static [S]>;
pub type SharedSamplesBuffer<S> = GenericBuffer<S, Arc<[S]>>;

/// A buffer of samples treated as a source.
#[derive(Clone)]
pub struct GenericBuffer<S, Container> {
    data: Container,
    position: usize,
    channels: u16,
    sample_rate: u32,
    duration: Duration,

    sample_type: PhantomData<S>,
}

impl<S, Container> GenericBuffer<S, Container>
where
    S: Sample,
    Container: AsRef<[S]>,
{
    /// Builds a new `SliceBuffer`.
    ///
    /// # Panic
    ///
    /// - Panics if the number of channels is zero.
    /// - Panics if the samples rate is zero.
    /// - Panics if the length of the buffer is larger than approximately 16 billion elements.
    ///   This is because the calculation of the duration would overflow.
    ///
    pub fn new(channels: u16, sample_rate: u32, data: Container) -> Self {
        assert!(channels != 0);
        assert!(sample_rate != 0);

        let duration_ns = 1_000_000_000u64
            .checked_mul(data.as_ref().len() as u64)
            .unwrap()
            / sample_rate as u64
            / channels as u64;
        let duration = Duration::new(
            duration_ns / 1_000_000_000,
            (duration_ns % 1_000_000_000) as u32,
        );

        Self {
            data,
            position: 0,
            channels,
            sample_rate,
            duration,
            sample_type: Default::default(),
        }
    }

    pub fn with_collector<Input, Collector>(input: Input, collector: Collector) -> Self
    where
        Input: Source<Item = S>,
        Collector: FnOnce(Input) -> Container,
    {
        let channels = input.channels();
        let sample_rate = input.sample_rate();
        Self::new(channels, sample_rate, collector(input))
    }

    pub fn reset(&mut self) {
        self.position = 0;
    }
}

impl<S, Container> GenericBuffer<S, Container>
where
    S: Sample,
    Container: AsRef<[S]> + FromIterator<S>,
{
    pub fn collect<Input>(input: Input) -> Self
    where
        Input: Source<Item = S>,
    {
        Self::with_collector(input, |input| input.collect())
    }
}

impl<S, Container> Source for GenericBuffer<S, Container>
where
    S: Sample + Clone,
    Container: AsRef<[S]>,
{
    #[inline]
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    #[inline]
    fn channels(&self) -> u16 {
        self.channels
    }

    #[inline]
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    #[inline]
    fn total_duration(&self) -> SourceDuration {
        SourceDuration::Exact(self.duration)
    }
}

impl<S, Container> Iterator for GenericBuffer<S, Container>
where
    S: Sample + Clone,
    Container: AsRef<[S]>,
{
    type Item = S;

    #[inline]
    fn next(&mut self) -> Option<S> {
        let value = self.data.as_ref().get(self.position).cloned();
        if value.is_some() {
            self.position += 1;
        }
        value
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let value = self.data.as_ref().len() - self.position;
        (value, Some(value))
    }
}

impl<S, Container> ExactSizeIterator for GenericBuffer<S, Container>
where
    S: Sample + Clone,
    Container: AsRef<[S]>,
{
}
