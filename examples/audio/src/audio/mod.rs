extern crate ffmpeg_next as ffmpeg;

use std::{ops::Range, path::Path};

use ffmpeg::{codec::context, ffi::AVCodecParameters, ChannelLayout};
use get_size::GetSize;
use rayon::{
    prelude::{
        IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelIterator,
    },
    slice::ParallelSlice,
};

mod device_output;
pub use device_output::*;

mod state;
pub use state::*;

#[repr(C)]
#[derive(Debug, Clone, GetSize)]
pub struct AudioSample<T, const CHANNELS: usize> {
    pub data: [T; CHANNELS],
}

#[derive(Debug, Clone)]
pub struct AudioSampleAverage<T, const CHANNELS: usize> {
    pub data: [T; CHANNELS],
    pub count: usize,
    pub index: usize,
}

impl<T: Default + Copy, const CHANNELS: usize> Default for AudioSample<T, CHANNELS> {
    fn default() -> Self {
        Self {
            data: [T::default(); CHANNELS],
        }
    }
}

impl<T, const CHANNELS: usize> Into<[T; CHANNELS]> for AudioSample<T, CHANNELS> {
    fn into(self) -> [T; CHANNELS] {
        self.data
    }
}

impl<T, const CHANNELS: usize> AudioSample<T, CHANNELS> {
    pub const fn num_channels(&self) -> usize {
        CHANNELS
    }
}

pub struct AudioSegment<T = f32, const CHANNELS: usize = 2> {
    pub samples: Vec<AudioSample<T, CHANNELS>>,
    pub sample_rate: u32,
}

impl<T, const CHANNELS: usize> AudioSegment<T, CHANNELS> {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            samples: Default::default(),
        }
    }

    pub const fn num_channels(&self) -> usize {
        CHANNELS
    }

    pub fn samples_packed(&self) -> &[T] {
        unsafe {
            std::slice::from_raw_parts(
                self.samples.as_ptr() as *const T,
                self.samples.len() * CHANNELS,
            )
        }
    }
}

impl AudioSegment<f32, 2> {
    pub fn chunked_average(&self, samples_per_pixel: usize) -> Vec<AudioSampleAverage<f32, 2>> {
        use rand::{seq::IteratorRandom, thread_rng};

        let samples_per_pixel_f = samples_per_pixel as f32;

        let sample_size = 120;
        let sample_size_f = sample_size as f32;

        // let mut rng = thread_rng();

        self.samples
            .par_chunks(samples_per_pixel)
            .enumerate()
            // .chunks(samples_per_pixel)
            .map_init(thread_rng, |rng, (i, c)| {
                let total = c.len();

                let averaged_sample = std::iter::once(AudioSample::default())
                    // .chain(
                    //     c.iter()
                    //         .choose_multiple(rng, sample_size)
                    //         .into_iter()
                    //         .cloned(),
                    // )
                    .chain(
                        c.chunks((total / sample_size).max(1))
                            .filter(|c| c.len() > 0)
                            .map(|c| c[0].clone()),
                    )
                    .reduce(|a, b| AudioSample {
                        data: [
                            a.data[0] + (b.data[0].abs() / sample_size_f),
                            a.data[1] + (b.data[1].abs() / sample_size_f),
                        ],
                    })
                    .unwrap_or_default();

                AudioSampleAverage {
                    data: averaged_sample.data,
                    count: c.len(),
                    index: i * samples_per_pixel,
                }
            })
            .collect()
    }
}

pub fn get_audio(stream_config: &cpal::StreamConfig) -> AudioSegment<f32, 2> {
    ffmpeg::init().unwrap();

    let mut ictx = ffmpeg::format::input(
        &Path::new(std::file!())
            .parent()
            .unwrap()
            // .join("Ante_Meridiam.mp3"),
            .join("alarm_beeps.wav"),
    )
    .unwrap();

    let audio = ictx.streams().best(ffmpeg::media::Type::Audio).unwrap();
    let audio_stream_index = audio.index();

    let context_decoder =
        ffmpeg::codec::context::Context::from_parameters(audio.parameters()).unwrap();

    let mut audio_decoder = context_decoder.decoder().audio().unwrap();
    let num_channels = audio_decoder.channels();

    unsafe {
        let channel_layout_ref = &mut (*audio_decoder.as_mut_ptr()).channel_layout;

        *channel_layout_ref = match channel_layout_ref {
            0 => ChannelLayout::default(num_channels as i32).bits(),
            _ => *channel_layout_ref,
        };
    }

    let mut resampler = ffmpeg::software::resampling::context::Context::get(
        audio_decoder.format(),
        audio_decoder.channel_layout(),
        audio_decoder.rate(),
        ffmpeg::format::Sample::F32(ffmpeg::format::sample::Type::Planar),
        audio_decoder.channel_layout(),
        stream_config.sample_rate.0,
    )
    .unwrap();

    let sample_rate = audio_decoder.rate();
    let format = audio_decoder.format();

    let mut audio_file = AudioSegment::<f32, 2>::new(sample_rate);

    // TODO: support resampling to device sample rate / format

    for (stream, packet) in ictx.packets() {
        if stream.index() == audio_stream_index {
            audio_decoder.send_packet(&packet).unwrap();

            let mut decoded = ffmpeg::frame::Audio::empty();

            while audio_decoder.receive_frame(&mut decoded).is_ok() {
                let mut resampled = ffmpeg::frame::Audio::empty();
                resampler.run(&decoded, &mut resampled).unwrap();

                assert_eq!(resampled.is_packed(), false);
                assert_eq!(resampled.is_corrupt(), false);
                assert_eq!(
                    resampled.format(),
                    ffmpeg::format::Sample::F32(ffmpeg::format::sample::Type::Planar),
                );
                assert_eq!(resampled.format().bytes(), 4);

                for (l, r) in std::iter::zip(
                    resampled.plane::<f32>(0).iter().copied(),
                    resampled.plane::<f32>(1).iter().copied(),
                ) {
                    audio_file.samples.push(AudioSample { data: [l, r] });
                }
            }
        }
    }

    println!(
        "total length: {:?}; heap size: {:?}",
        std::time::Duration::from_secs(audio_file.samples.len() as u64 / sample_rate as u64),
        audio_file.samples.get_heap_size() as f32 * 10e-7
    );

    audio_file
}
