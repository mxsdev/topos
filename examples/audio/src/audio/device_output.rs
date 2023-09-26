use std::sync::Arc;

use cpal::platform::CoreAudioStream;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat, SizedSample, StreamConfig};
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

pub fn init_audio_buffer<
    T: SizedSample + Default + Send + 'static,
    R: ring_buffer::RbRef + Send + 'static,
>(
    // desired_sample_rate: Option<u32>,
    mut ring_buffer: ringbuf::Consumer<T, R>,
) -> (cpal::Stream, cpal::StreamConfig)
where
    R::Rb: ring_buffer::RbRead<T>,
{
    let (device, stream_config) = init_cpal();

    let stream = device
        .build_output_stream(
            &stream_config,
            move |data, _| {
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
        .expect("error building stream");

    stream.play().expect("error playing stream");

    (stream, stream_config)
}
