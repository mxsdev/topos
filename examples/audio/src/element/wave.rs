use std::{mem::MaybeUninit, sync::Arc};

use rayon::prelude::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use ringbuf::{HeapRb, LocalRb};
use topos::{
    accessibility::AccessNodeBuilder,
    color::ColorRgba,
    element::{Element, Response},
    input::{input_state::InputState, Key},
    math::{CompleteScaleFactor, Pos, Rect, Size},
    scene::{
        ctx::SceneContext,
        layout::{FlexBox, LayoutPass, LayoutPassResult, Percent},
    },
    shape::PaintRectangle,
};

use crate::{
    audio::{
        self, new_state_bidi, AudioPlayState, AudioPlayStatePaused, AudioPlayStatePlaying,
        AudioSample, AudioSampleAverage, AudioSegment, AudioState, AudioStateProducer, Stream,
        StreamTime,
    },
    cache::CachedValue,
};

pub struct Wave {
    stream: Stream,
    response: Response<Rect>,
    audio_file: Arc<AudioSegment>,
    samples_averaged_cache: CachedValue<Vec<AudioSampleAverage<f32, 2>>, usize>,
    audio_state: AudioStateProducer,
}

// type StreamBuffer = LocalRb<f32, Vec<MaybeUninit<f32>>>;
type StreamBuffer = HeapRb<f32>;

impl Wave {
    pub fn new() -> Self {
        let buffer = StreamBuffer::new(1024);
        let (mut producer, consumer) = buffer.split();

        let stream = audio::init_audio_buffer(consumer);

        let audio_file: Arc<_> = audio::get_audio(stream.stream_config()).into();
        println!("audio sample_rate: {:?}", audio_file.sample_rate);

        let initial_audio_state = AudioState {
            play_state: AudioPlayState::Stopped,
            volume: 0.1,
        };
        let (mut audio_state_consumer, audio_state_producer) = new_state_bidi(initial_audio_state);

        {
            let audio_file = audio_file.clone();
            let stream_time = stream.stream_time();

            std::thread::spawn(move || {
                let mut last_sample: Option<usize> = None;

                loop {
                    let audio_state = audio_state_consumer.get_state();
                    let nc = audio_file.num_channels();

                    match audio_state.play_state {
                        AudioPlayState::Playing(play_state) => {
                            let last_sample_ref = last_sample.get_or_insert_with(|| {
                                play_state
                                    .pos_now(&stream_time)
                                    .to_sample_time(audio_file.sample_rate)
                                    .inner
                                    .sample_idx
                            });

                            let samples_to_write = producer.free_len() / audio_file.num_channels();

                            if samples_to_write > 0 {
                                let samples = &audio_file.samples_packed()[(*last_sample_ref * nc)
                                    ..(*last_sample_ref + samples_to_write) * nc];

                                // turn down volume
                                let samples = samples
                                    .iter()
                                    .map(|s| s * audio_state.volume)
                                    .collect::<Vec<_>>();

                                producer.push_slice(&samples);
                            }

                            *last_sample_ref += samples_to_write;
                        }
                        AudioPlayState::Paused(_) | AudioPlayState::Stopped => {
                            last_sample = None;
                        }
                    }
                }
            });
        }

        Self {
            stream,
            response: Default::default(),
            audio_file,
            samples_averaged_cache: Default::default(),
            audio_state: audio_state_producer,
        }
    }
}

impl Wave {
    fn populate_sample_cache(
        &mut self,
        rect: Rect,
        sf: CompleteScaleFactor,
    ) -> (&Vec<AudioSampleAverage<f32, 2>>, f32) {
        let sf = sf.get().into_inner();

        let pixel_size = 1. / sf;
        let samples_per_pixel = self.audio_file.samples.len() / (rect.width() * sf).ceil() as usize;
        // let samples_per_pixel_f = samples_per_pixel as f32;

        let samples_averaged = self
            .samples_averaged_cache
            .get_or_insert_with(samples_per_pixel, || {
                self.audio_file.chunked_average(samples_per_pixel)
            });

        (samples_averaged, pixel_size)
    }
}

impl Element for Wave {
    fn layout(&mut self, layout_pass: &mut LayoutPass) -> LayoutPassResult {
        let result = layout_pass
            .engine()
            .new_leaf(
                FlexBox::builder()
                    .width(Percent(1.))
                    .max_height(200.)
                    .height(Percent(1.)),
            )
            .unwrap();

        result
    }

    fn layout_post(&mut self, resources: &mut topos::scene::scene::SceneResources, rect: Rect) {
        self.populate_sample_cache(rect, resources.scale_factor());
    }

    fn input(&mut self, input: &mut InputState, rect: Rect) {
        self.response.update_rect(input, rect);

        if input.key_pressed(Key::Space) {
            println!("pressed space!");
            self.audio_state.modify_state(|mut state| {
                state.play_state = match state.play_state {
                    AudioPlayState::Playing(state_playing) => {
                        AudioPlayState::Paused(AudioPlayStatePaused::new(
                            state_playing.pos_now(&self.stream.stream_time_ref()),
                        ))
                    }
                    AudioPlayState::Paused(state_paused) => {
                        AudioPlayState::Playing(AudioPlayStatePlaying::new_now(
                            state_paused.pos,
                            &self.stream.stream_time_ref(),
                        ))
                    }
                    AudioPlayState::Stopped => {
                        AudioPlayState::Playing(AudioPlayStatePlaying::new_now(
                            std::time::Duration::default(),
                            &self.stream.stream_time_ref(),
                        ))
                    }
                };

                state
            })
        }

        if input.key_pressed(Key::ArrowUp) {
            println!("pressed arrow up!");
            self.audio_state.modify_state(|mut state| {
                state.volume += 0.1;
                state
            });
        }

        if input.key_pressed(Key::ArrowDown) {
            println!("pressed arrow down!");
            self.audio_state.modify_state(|mut state| {
                state.volume -= 0.1;
                state
            });
        }
    }

    fn ui(&mut self, ctx: &mut SceneContext, rect: Rect) {
        let playhead_pos = self
            .audio_state
            .get_state()
            .play_state
            .pos_now(&self.stream.stream_time_ref())
            .unwrap_or_default()
            .to_sample_time(self.audio_file.sample_rate)
            .inner
            .sample_idx;

        let (samples_averaged, pixel_size) = self.populate_sample_cache(rect, ctx.scale_factor());

        const WAVE_COLOR: ColorRgba = ColorRgba::new(1., 1., 1., 0.3);
        const WAVE_COLOR_PLAYHEAD: ColorRgba = ColorRgba::new(0., 0., 1., 0.3);

        for (
            i,
            AudioSampleAverage {
                data: [l, r],
                index,
                ..
            },
        ) in samples_averaged.iter().enumerate()
        {
            let is_behind_playhead = *index < playhead_pos;
            let color = if is_behind_playhead {
                WAVE_COLOR_PLAYHEAD
            } else {
                WAVE_COLOR
            };

            {
                let rect_height = rect.height() * *l;

                ctx.add_shape(
                    PaintRectangle::from_rect(Rect::from_min_size(
                        Pos::new(i as f32 * pixel_size, (rect.height() - rect_height) / 2.)
                            + rect.min.to_vector(),
                        Size::new(pixel_size, rect_height),
                    ))
                    .with_fill(color),
                );
            }

            {
                let rect_height = rect.height() * *r;

                ctx.add_shape(
                    PaintRectangle::from_rect(Rect::from_min_size(
                        Pos::new(i as f32 * pixel_size, (rect.height() - rect_height) / 2.)
                            + rect.min.to_vector(),
                        Size::new(pixel_size, rect_height),
                    ))
                    .with_fill(color),
                );
            }
        }
    }

    fn node(&self) -> AccessNodeBuilder {
        AccessNodeBuilder::new(Default::default())
    }
}
