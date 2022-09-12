use crate::{Sample, Source};
use cpal::Sample as CPSample;
use std::fmt::Debug;
use std::time::Duration;

pub trait MonoMapper: Clone + Debug {
    type Args;

    fn new(args: &Self::Args) -> Self;
    fn feed(&mut self, sample: f32, channels: u16, channel: u16);
    fn finish(self, channels: u16) -> f32;
}

#[derive(Clone, Debug)]
pub struct AverageMapper(f32);
impl MonoMapper for AverageMapper {
    type Args = ();

    fn new(_args: &Self::Args) -> Self {
        Self(0.0)
    }

    fn feed(&mut self, sample: f32, _channels: u16, _channel: u16) {
        self.0 += sample;
    }

    fn finish(self, channels: u16) -> f32 {
        self.0 / (channels as f32)
    }
}

#[derive(Clone, Debug)]
pub struct SingleChannelMapper {
    channel: u16,
    value: f32,
}
impl MonoMapper for SingleChannelMapper {
    type Args = u16;

    fn new(channel: &Self::Args) -> Self {
        Self {
            channel: *channel,
            value: 0.0,
        }
    }

    fn feed(&mut self, sample: f32, _channels: u16, channel: u16) {
        if self.channel == channel {
            self.value = sample;
        }
    }

    fn finish(self, _channels: u16) -> f32 {
        self.value
    }
}

#[derive(Clone, Debug)]
pub struct GenericMono<M, I>
where
    M: MonoMapper,
    I: Source,
    I::Item: Sample,
{
    args: M::Args,
    input: I,
}

pub type Mono<I> = GenericMono<AverageMapper, I>;
pub type SingleChannelMono<I> = GenericMono<SingleChannelMapper, I>;

impl<I> Mono<I>
where
    I: Source,
    I::Item: Sample,
{
    pub fn new(inner: I) -> Self {
        Self {
            args: (),
            input: inner,
        }
    }
}
impl<I> SingleChannelMono<I>
where
    I: Source,
    I::Item: Sample,
{
    pub fn new(inner: I, channel: u16) -> Self {
        assert!(
            channel < inner.channels(),
            "{} is out of bounds channel ({})",
            channel,
            inner.channels()
        );

        Self {
            args: channel,
            input: inner,
        }
    }
}

impl<M, I> GenericMono<M, I>
where
    M: MonoMapper,
    I: Source,
    I::Item: Sample,
{
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

    fn map_size_hint(&self, samples: usize) -> usize {
        samples / (self.input.channels() as usize)
    }
}

impl<M, I> Iterator for GenericMono<M, I>
where
    M: MonoMapper,
    I: Source,
    I::Item: Sample,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let channels = self.input.channels();

        if channels == 1 {
            return self.input.next();
        }

        let mut mapper = M::new(&self.args);
        for channel in 0..channels {
            let v = self.input.next()?;
            mapper.feed(v.to_f32(), channels, channel);
        }

        Some(<I::Item as CPSample>::from(&mapper.finish(channels)))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (min, max) = self.input.size_hint();
        (self.map_size_hint(min), max.map(|v| self.map_size_hint(v)))
    }
}

impl<M, I> Source for GenericMono<M, I>
where
    M: MonoMapper,
    I: Source,
    I::Item: Sample,
{
    fn current_frame_len(&self) -> Option<usize> {
        self.input
            .current_frame_len()
            .map(|v| self.map_size_hint(v))
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        self.input.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.input.total_duration()
    }
}
