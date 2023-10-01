use ringbuffer::{ConstGenericRingBuffer, RingBuffer, RingBufferExt, RingBufferWrite};

use num_traits::Pow;

const FRAMEPACER_NUM_SAMPLES: usize = 120;

pub type FramepacerInstant = wgpu::PresentationTimestamp;

pub trait TimeWithAdapter: RelativeDuration {
    fn now(adapter: &wgpu::Adapter) -> Self;

    fn elapsed(&self, adapter: &wgpu::Adapter) -> std::time::Duration
    where
        Self: Sized,
    {
        Self::now(adapter).duration_since(self)
    }
}

impl TimeWithAdapter for FramepacerInstant {
    fn now(adapter: &wgpu::Adapter) -> Self {
        adapter.get_presentation_timestamp()
    }
}

pub trait RelativeDuration {
    fn duration_since(&self, earlier: &Self) -> std::time::Duration;
}

impl RelativeDuration for FramepacerInstant {
    fn duration_since(&self, earlier: &Self) -> std::time::Duration {
        std::time::Duration::from_nanos((self.0 - earlier.0).try_into().unwrap())
    }
}

#[derive(Default)]
pub struct Framepacer {
    // time in seconds
    last_30: ConstGenericRingBuffer<f64, FRAMEPACER_NUM_SAMPLES>,
    i: usize,

    worst_frametime_secs: f64,

    deadline: Option<FramepacerInstant>,

    last_refresh_rate_nanos: Option<u128>,
    last_presentation_start: Option<FramepacerInstant>,
}

const DEFAULT_FRAME_TIME_SECS: f64 = 1. / 60.;

const DEVIATION_BUFFER_MICROS: u64 = 30;

impl Framepacer {
    pub fn new() {
        Default::default()
    }

    pub fn start_window(
        &mut self,
        presentation_start: FramepacerInstant,
        frame_time_nanos: Option<u128>,
    ) {
        // if let Some((last_presentation_start, frame_time_nanos)) =
        //     Option::zip(self.last_presentation_start.take(), frame_time_nanos)
        // {
        //     let del = presentation_start
        //         .0
        //         .saturating_sub(last_presentation_start.0);

        //     let diff = del as i128 - frame_time_nanos as i128;

        //     // let diff = u128::max(
        //     //     del.saturating_sub(frame_time_nanos),
        //     //     frame_time_nanos.saturating_sub(del),
        //     // );

        //     println!("off by {}ns", diff);
        // }

        // self.last_presentation_start = Some(presentation_start);

        let frame_time_nanos = frame_time_nanos.unwrap_or_else(|| {
            std::time::Duration::from_secs_f64(DEFAULT_FRAME_TIME_SECS).as_nanos()
        });

        self.last_refresh_rate_nanos = frame_time_nanos.into();

        self.deadline = wgpu::PresentationTimestamp(presentation_start.0 + frame_time_nanos).into();
    }

    pub fn check_missed_deadline(
        &mut self,
        now: FramepacerInstant,
        render_time: Option<std::time::Duration>,
    ) -> bool {
        if let Some(deadline) = self.deadline {
            let missed = now > deadline;

            if missed {
                log::debug!(
                    "missed deadline by {:?}!",
                    std::time::Duration::from_nanos((now.0 - deadline.0).try_into().unwrap())
                );

                let predicted_frametime =
                    crate::time::Duration::from_secs_f64(self.worst_frametime_secs)
                        + crate::time::Duration::from_micros(DEVIATION_BUFFER_MICROS);

                if let Some(render_time) = render_time {
                    log::debug!(
                        "\trender time: {:?}, anticipated: {:?}",
                        render_time,
                        predicted_frametime
                    );
                }
            }

            missed
        } else {
            false
        }
    }

    pub fn get_deadline(&self) -> Option<FramepacerInstant> {
        self.deadline
    }

    pub fn should_render(&mut self, start_time: FramepacerInstant) -> (bool, FramepacerInstant) {
        let should_render = match self.deadline {
            Some(deadline) => {
                let predicted_finish_time = wgpu::PresentationTimestamp(
                    start_time.0
                        + (crate::time::Duration::from_secs_f64(self.worst_frametime_secs)
                            + crate::time::Duration::from_micros(DEVIATION_BUFFER_MICROS))
                        .as_nanos(),
                );

                // TODO: add buffer here for input/parsing time...
                predicted_finish_time >= deadline
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

    pub fn refresh_rate_nanos(&self) -> u128 {
        self.last_refresh_rate_nanos.unwrap_or_else(|| {
            std::time::Duration::from_secs_f64(DEFAULT_FRAME_TIME_SECS).as_nanos()
        })
    }
}
