pub use instant::{Duration, Instant, SystemTime};

use euclid::num::Floor;

pub struct FramerateCounter {
    start: Instant,

    framerate: u32,
    tick_ct: u32,

    last_tick: Option<Instant>,
    last_frame_time: Option<Duration>,
}

impl Default for FramerateCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl FramerateCounter {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            framerate: 0,
            tick_ct: 0,
            last_tick: None,
            last_frame_time: None,
        }
    }

    pub fn tick(&mut self) -> Option<Duration> {
        let tick_time = Instant::now();
        let time_since_last_tick = match self.last_tick {
            Some(last_tick) => {
                let elapsed_time = (tick_time - self.start).as_secs();
                let last_elapsed_time = (last_tick - self.start).as_secs();

                if elapsed_time.floor() != last_elapsed_time.floor() {
                    self.framerate = self.tick_ct;
                    self.tick_ct = 0;
                }

                Some(tick_time - last_tick)
            }
            None => None,
        };

        self.tick_ct += 1;

        self.last_tick = Some(tick_time);
        self.last_frame_time = time_since_last_tick;

        return time_since_last_tick;
    }

    pub fn framerate(&self) -> u32 {
        self.framerate
    }
}
