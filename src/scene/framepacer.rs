use ringbuffer::{ConstGenericRingBuffer, RingBuffer, RingBufferExt, RingBufferWrite};

#[derive(Default)]
pub struct Framepacer {
    last_30: ConstGenericRingBuffer<std::time::Duration, 30>,
    i: usize,
}

impl Framepacer {
    pub fn new() {
        Default::default()
    }

    pub fn push_frametime(&mut self, duration: std::time::Duration) {
        self.last_30.push(duration);
        self.i += 1;

        if self.i >= 30 {
            let mut total = std::time::Duration::default();

            for duration in self.last_30.iter() {
                total += *duration;
            }

            total /= self.last_30.len() as u32;

            log::trace!("average frame-time: {:?}", total);

            self.i = 0;
        }
    }
}
