use std::collections::VecDeque;
use std::ops::Add;
use std::sync::atomic::{AtomicI32, AtomicI64, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

use cpal::platform::CoreAudioStream;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat, SizedSample, StreamConfig, StreamInstant};
use ringbuf::{ring_buffer, Rb};

use super::{AudioSample, AudioSegment};

fn init_cpal() -> (cpal::Device, cpal::StreamConfig) {
    let device = cpal::default_host()
        .default_output_device()
        .expect("no output device available");

    // Create an output stream for the audio so we can play it
    // NOTE: If system doesn't support the file's sample rate, the program will panic when we try to play,
    //       so we'll need to resample the audio to a supported config
    let supported_config_range = device
        .supported_output_configs()
        .expect("error querying audio output configs")
        .next()
        .expect("no supported audio config found")
        .with_max_sample_rate();
    // .find(|s| {
    //     s.max_sample_rate().0 >= desired_sample_rate
    //         && s.min_sample_rate().0 <= desired_sample_rate
    //         && s.sample_format() == SampleFormat::F32
    // })
    // .expect("no supported config for desired sample rate")
    // .with_sample_rate(cpal::SampleRate(desired_sample_rate));

    println!("device name: {}", device.name().unwrap());

    // Pick the best (highest) sample rate
    (device, supported_config_range.config())
}

pub trait StreamInstantTrait {
    fn duration_since(&self, earlier: &Self) -> Option<std::time::Duration>;
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct StreamInstantCopy {
    secs: i64,
    nanos: u32,
}

impl From<StreamInstant> for StreamInstantCopy {
    fn from(stream_instant: StreamInstant) -> Self {
        let stream_instant_copied: StreamInstantCopy =
            unsafe { std::mem::transmute(stream_instant) };

        Self {
            secs: stream_instant_copied.secs,
            nanos: stream_instant_copied.nanos,
        }
    }
}

impl StreamInstantCopy {
    fn as_nanos(&self) -> i128 {
        (self.secs as i128 * 1_000_000_000) + self.nanos as i128
    }
}

impl StreamInstantTrait for StreamInstantCopy {
    /// The amount of time elapsed from another instant to this one.
    ///
    /// Returns `None` if `earlier` is later than self.
    fn duration_since(&self, earlier: &Self) -> Option<std::time::Duration> {
        if self < earlier {
            None
        } else {
            (self.as_nanos() - earlier.as_nanos())
                .try_into()
                .ok()
                .map(std::time::Duration::from_nanos)
        }
    }
}

impl StreamInstantTrait for cpal::StreamInstant {
    fn duration_since(&self, earlier: &Self) -> Option<std::time::Duration> {
        cpal::StreamInstant::duration_since(self, earlier)
    }
}

pub struct AtomicStreamInstant {
    pub secs: AtomicI64,
    pub nanos: AtomicU32,
}

impl<X: Into<StreamInstantCopy>> From<X> for AtomicStreamInstant {
    fn from(stream_instant: X) -> Self {
        let stream_instant = stream_instant.into();

        Self {
            secs: AtomicI64::new(stream_instant.secs),
            nanos: AtomicU32::new(stream_instant.nanos),
        }
    }
}

impl Default for AtomicStreamInstant {
    fn default() -> Self {
        Self {
            secs: AtomicI64::new(0),
            nanos: AtomicU32::new(0),
        }
    }
}

impl AtomicStreamInstant {
    const ORDERING: std::sync::atomic::Ordering = std::sync::atomic::Ordering::SeqCst;

    pub fn update(&self, stream_instant: impl Into<StreamInstantCopy>) {
        let stream_instant = stream_instant.into();

        self.secs.store(stream_instant.secs, Self::ORDERING);
        self.nanos.store(stream_instant.nanos, Self::ORDERING);
    }

    pub fn retrieve(&self) -> StreamInstantCopy {
        StreamInstantCopy {
            secs: self.secs.load(Self::ORDERING).into(),
            nanos: self.nanos.load(Self::ORDERING).into(),
        }
    }
}

#[derive(Clone, Default)]
pub struct StreamTime {
    num_played_samples: Arc<AtomicU64>,
    sample_rate: u32,
}

impl StreamTime {
    const ORDERING: std::sync::atomic::Ordering = std::sync::atomic::Ordering::SeqCst;

    pub fn add_samples(&self, num_played_samples: u64) {
        self.num_played_samples
            .fetch_add(num_played_samples, Self::ORDERING);
    }

    pub fn retrieve(&self) -> u64 {
        self.num_played_samples.load(Self::ORDERING)
    }

    pub fn now(&self) -> StreamTimeStamp {
        std::time::Duration::from_secs_f64(self.retrieve() as f64 / self.sample_rate as f64)
    }
}

pub struct Stream {
    cpal_stream: cpal::Stream,
    cpal_stream_config: cpal::StreamConfig,
    played_samples: StreamTime,
}

pub type StreamTimeStamp = std::time::Duration;

impl Stream {
    pub fn new(
        cpal_stream: cpal::Stream,
        cpal_stream_config: cpal::StreamConfig,
        samples: Option<StreamTime>,
    ) -> Self {
        let mut played_samples = samples.unwrap_or_default();
        played_samples.sample_rate = cpal_stream_config.sample_rate.0;

        Self {
            cpal_stream,
            cpal_stream_config,
            played_samples,
        }
    }

    pub fn stream_config(&self) -> &cpal::StreamConfig {
        &self.cpal_stream_config
    }

    pub fn sample_rate(&self) -> u32 {
        self.cpal_stream_config.sample_rate.0
    }

    pub fn now(&self) -> StreamTimeStamp {
        self.played_samples.now()
    }

    pub fn stream_time(&self) -> StreamTime {
        self.played_samples.clone()
    }

    pub fn stream_time_ref(&self) -> &StreamTime {
        &self.played_samples
    }
}

pub fn init_audio_buffer<
    T: SizedSample + Default + Send + 'static,
    R: ring_buffer::RbRef + Send + 'static,
>(
    // desired_sample_rate: Option<u32>,
    mut ring_buffer: ringbuf::Consumer<T, R>,
) -> Stream
where
    R::Rb: ring_buffer::RbRead<T>,
{
    let (device, stream_config) = init_cpal();

    let played_samples = StreamTime::default();

    let stream = {
        let played_samples = played_samples.clone();
        let num_channels = stream_config.channels as usize;

        let mut sample_add_queue = VecDeque::<(u64, StreamInstant)>::new();

        device
            .build_output_stream(
                &stream_config,
                move |data, info| {
                    while sample_add_queue
                        .front()
                        .map(|(_, i)| i.duration_since(&info.timestamp().callback).is_none())
                        .unwrap_or_default()
                    {
                        let (num_samples, _) = sample_add_queue.pop_front().unwrap();
                        played_samples.add_samples(num_samples);
                    }

                    assert_eq!(data.len() % num_channels, 0);

                    // TODO: better scheduling with separate thread
                    sample_add_queue.push_back((
                        (data.len() / num_channels) as u64,
                        info.timestamp().playback,
                    ));

                    // Fill the buffer with samples
                    for sample in data.iter_mut() {
                        *sample = ring_buffer.pop().unwrap_or_default();

                        // match ring_buffer.pop() {
                        //     Some(value) => *sample = value,
                        //     None => {
                        //         *sample = {
                        //             log::error!("no samples available!!");
                        //             T::default()
                        //         }
                        //     }
                        // }
                    }
                },
                |err| {
                    eprintln!("an error occurred on stream: {}", err);
                },
                None,
            )
            .expect("error building stream")
    };

    stream.play().expect("error playing stream");

    Stream::new(stream, stream_config, played_samples.into())
}
