use crate::source::{Amplify, SourceDuration, SourceUtils};
use crate::{Sample, Source};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct StoppableFade<I> {
    input: Amplify<I>,
    state: StopState,
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum StopState {
    Playing,
    Fading {
        left: u64,
        reduction_per_sample: f32,
    },
    Stopped,
}

impl<I> StoppableFade<I>
where
    I: Source,
    I::Item: Sample,
{
    pub fn new(input: I) -> Self {
        Self {
            input: input.amplify(1.0),
            state: StopState::Playing,
        }
    }

    /// Returns a reference to the inner source.
    #[inline]
    pub fn inner(&self) -> &I {
        self.input.inner()
    }

    /// Returns a mutable reference to the inner source.
    #[inline]
    pub fn inner_mut(&mut self) -> &mut I {
        self.input.inner_mut()
    }

    /// Returns the inner source.
    #[inline]
    pub fn into_inner(self) -> I {
        self.input.into_inner()
    }
}

impl<I> StoppableFade<I>
where
    I: Source,
    I::Item: Sample,
{
    /// Stops the sound.
    #[inline]
    pub fn stop(&mut self, fade: Duration) {
        if self.state == StopState::Playing {
            let fade_samples =
                (fade.as_nanos() as u64) / (self.duration_per_sample().as_nanos() as u64);
            let reduction_per_sample = 1.0 / (fade_samples as f32);

            self.state = StopState::Fading {
                left: fade_samples,
                reduction_per_sample,
            };
        }
    }
}

impl<I> Iterator for StoppableFade<I>
where
    I: Source,
    I::Item: Sample,
{
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<I::Item> {
        match self.state {
            StopState::Playing => self.input.next(),
            StopState::Stopped => None,
            StopState::Fading {
                left,
                reduction_per_sample,
            } => {
                if left == 0 {
                    self.state = StopState::Stopped;
                    return self.next();
                }

                self.state = StopState::Fading {
                    left: left - 1,
                    reduction_per_sample,
                };

                self.input
                    .set_factor(self.input.factor() - reduction_per_sample);

                self.input.next()
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.input.size_hint()
    }
}

impl<I> Source for StoppableFade<I>
where
    I: Source,
    I::Item: Sample,
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
    fn total_duration(&self) -> SourceDuration {
        self.input.total_duration()
    }
}
