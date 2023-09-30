use std::sync::mpsc;

use cpal::StreamInstant;

use super::{Stream, StreamInstantCopy, StreamInstantTrait, StreamTime, StreamTimeStamp};

// trait AudioPosUnit {
//     type Time: Copy + Clone;
// }

// #[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
// struct Duration;

// impl AudioPosUnit for Duration {
//     type Time = StreamTimeStamp;
// }

// #[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
// struct Sample;

#[derive(Clone, Copy)]
pub struct SampleTime {
    pub sample_idx: usize,
    pub sample_rate: u32,
}

// impl AudioPosUnit for Sample {
//     type Time = SampleTime;
// }

#[derive(Clone, Copy, Default, Debug)]
pub struct AudioPos<TimeUnit = StreamTimeStamp> {
    pub inner: TimeUnit,
}

impl<TimeUnit> From<TimeUnit> for AudioPos<TimeUnit> {
    fn from(inner: TimeUnit) -> Self {
        AudioPos { inner }
    }
}

impl AudioPos<StreamTimeStamp> {
    pub fn to_sample_time(&self, sample_rate: u32) -> AudioPos<SampleTime> {
        AudioPos {
            inner: SampleTime {
                sample_idx: (self.inner.as_secs_f64() * sample_rate as f64).round() as usize,
                sample_rate,
            },
        }
    }
}

impl AudioPos<SampleTime> {
    pub fn to_duration(&self) -> AudioPos<StreamTimeStamp> {
        AudioPos {
            inner: StreamTimeStamp::from_secs_f64(
                self.inner.sample_idx as f64 / self.inner.sample_rate as f64,
            ),
        }
    }
}

// pub type AudioPos = StreamTimeStamp;

#[derive(Clone, Copy)]
pub struct AudioPlayStatePlaying {
    pub started_at: StreamTimeStamp,
    pub pos: AudioPos,
}

impl AudioPlayStatePlaying {
    pub fn new(started_at: StreamTimeStamp, pos: impl Into<AudioPos>) -> Self {
        Self {
            started_at,
            pos: pos.into(),
        }
    }

    pub fn new_now(pos: impl Into<AudioPos>, stream: &StreamTime) -> Self {
        Self::new(stream.now(), pos)
    }

    pub fn pos_at(&self, t: StreamTimeStamp) -> AudioPos {
        return (self.pos.inner + (t - self.started_at)).into();
    }

    pub fn pos_now(&self, stream_time: &StreamTime) -> AudioPos {
        return self.pos_at(stream_time.now());
    }
}

#[derive(Clone, Copy)]
pub struct AudioPlayStatePaused {
    pub pos: AudioPos,
}

impl AudioPlayStatePaused {
    pub fn new(pos: impl Into<AudioPos>) -> Self {
        Self { pos: pos.into() }
    }
}

#[derive(Clone, Copy)]
pub enum AudioPlayState {
    Playing(AudioPlayStatePlaying),
    Paused(AudioPlayStatePaused),
    Stopped,
}

impl AudioPlayState {
    pub fn pos_now(&self, stream_time: &StreamTime) -> Option<AudioPos> {
        match self {
            AudioPlayState::Playing(play_state) => play_state.pos_now(stream_time).into(),
            AudioPlayState::Paused(pause_state) => pause_state.pos.into(),
            AudioPlayState::Stopped => None,
        }
    }
}

#[derive(Clone, Copy)]
pub struct AudioState {
    pub play_state: AudioPlayState,
    pub volume: f32,
}

pub struct AudioStateProducer<State: Copy = AudioState> {
    state: State,
    sender: mpsc::Sender<State>,
}

impl<State: Copy> AudioStateProducer<State> {
    pub fn get_state(&self) -> State {
        self.state
    }

    pub fn push_state(&mut self, state: State) {
        self.state = state;
        self.sender.send(state).unwrap();
    }

    pub fn modify_state(&mut self, f: impl FnOnce(State) -> State) {
        self.push_state(f(self.get_state()));
    }
}

pub struct AudioStateConsumer<State: Copy = AudioState> {
    state: State,
    receiver: mpsc::Receiver<State>,
}

impl<State: Copy> AudioStateConsumer<State> {
    pub fn get_state(&mut self) -> State {
        for state in self.receiver.try_iter() {
            self.state = state;
        }

        return self.state;
    }
}

pub fn new_state_bidi<State: Copy>(
    initial_state: State,
) -> (AudioStateConsumer<State>, AudioStateProducer<State>) {
    let (sender, receiver) = mpsc::channel();

    (
        AudioStateConsumer {
            state: initial_state,
            receiver,
        },
        AudioStateProducer {
            state: initial_state,
            sender,
        },
    )
}
