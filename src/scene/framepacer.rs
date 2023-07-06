use ringbuffer::{ConstGenericRingBuffer, RingBuffer, RingBufferExt, RingBufferWrite};

use num_traits::Pow;

const FRAMEPACER_NUM_SAMPLES: usize = 120;

#[derive(Default)]
pub struct Framepacer {
    // time in seconds
    last_30: ConstGenericRingBuffer<f64, FRAMEPACER_NUM_SAMPLES>,
    i: usize,

    worst_frametime_secs: f64,

    deadline: Option<crate::time::Instant>,
}

const DEFAULT_FRAME_TIME: f64 = 1. / 60.;

impl Framepacer {
    pub fn new() {
        Default::default()
    }

    pub fn start_window(&mut self, start: crate::time::Instant, frame_time_secs: Option<f64>) {
        let frame_time =
            crate::time::Duration::from_secs_f64(frame_time_secs.unwrap_or(DEFAULT_FRAME_TIME));

        self.deadline = (start + frame_time).into();
    }

    pub fn check_missed_deadline(&mut self, now: crate::time::Instant) -> bool {
        let missed = if let Some(deadline) = self.deadline {
            let missed = now > deadline;

            if missed {
                log::debug!("missed deadline by {:?}!", now - deadline);
            }

            missed
        } else {
            false
        };

        missed
    }

    pub fn should_render(&mut self) -> (bool, crate::time::Instant) {
        let start_time = crate::time::Instant::now();

        let should_render = match self.deadline {
            Some(deadline) => {
                // TODO: add buffer here for input/parsing time...
                start_time
                    + crate::time::Duration::from_secs_f64(self.worst_frametime_secs)
                    + crate::time::Duration::from_micros(700)
                    >= deadline
            }

            None => true,
        };

        (should_render, start_time)
    }

    // pub fn next_deadline(&mut self, from: crate::time::Instant) -> crate::time::Instant {}

    pub fn push_frametime(&mut self, duration: crate::time::Duration) {
        let secs = duration.as_secs_f64();

        self.last_30.push(secs);
        self.i += 1;

        let N = self.last_30.len() as f64;

        // log::trace!("buffer size: {:?}", N);

        if N <= 2. {
            self.worst_frametime_secs = 10.;
            return;
        }

        let mu = self.last_30.iter().copied().sum::<f64>() / N;

        let sigma = self
            .last_30
            .iter()
            .copied()
            .map(|x| ((x - mu).pow(2) / (N - 1.)))
            .sum::<f64>()
            .sqrt();

        self.worst_frametime_secs = mu + 3. * sigma;

        // crate::time::Duration::as_secs_f64();

        // crate::time::Duration

        // let mut total = crate::time::Duration::default();

        // for duration in self.last_30.iter() {
        //     total += *duration;
        // }

        // let mu = total / self.last_30.len() as u32;

        if self.i >= 30 {
            log::trace!(
                "worst case: {:?}, mu: {:?}, sigma: {:?}",
                crate::time::Duration::from_secs_f64(self.worst_frametime_secs),
                crate::time::Duration::from_secs_f64(mu),
                crate::time::Duration::from_secs_f64(sigma),
            );
            self.i = 0;
        }
    }
}
