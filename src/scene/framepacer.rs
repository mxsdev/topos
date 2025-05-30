use ringbuffer::{ConstGenericRingBuffer, RingBuffer};

use num_traits::Pow;

use crate::{app::get_window_last_screen_draw_time, surface::RenderingContext};

const FRAMEPACER_NUM_SAMPLES: usize = 120;

pub type FramepacerInstant = wgpu::PresentationTimestamp;

pub trait IntoDuration {
    fn into_duration(self) -> std::time::Duration;
}

impl IntoDuration for FramepacerInstant {
    fn into_duration(self) -> std::time::Duration {
        std::time::Duration::from_nanos(self.0.try_into().unwrap())
    }
}

pub trait InstantLike: PartialOrd {
    type Context;

    fn now(adapter: &Self::Context) -> Self;

    fn context_from(render_ctx: &RenderingContext) -> &Self::Context
    where
        Self: Sized;

    fn elapsed(&self, adapter: &Self::Context) -> std::time::Duration
    where
        Self: Sized,
    {
        Self::now(adapter).duration_since(self)
    }

    fn duration_since(&self, earlier: &Self) -> std::time::Duration;

    fn add_duration(self, duration: std::time::Duration) -> Self;

    fn query_presentation_statistics(
        _surface: &wgpu::Surface,
        _window: &winit::window::Window,
        fallback: Self,
    ) -> Self
    where
        Self: Sized,
    {
        fallback
    }
}

impl InstantLike for FramepacerInstant {
    type Context = RenderingContext;

    fn now(RenderingContext { adapter, .. }: &Self::Context) -> Self {
        adapter.get_presentation_timestamp()
    }

    fn context_from(render_ctx: &RenderingContext) -> &Self::Context
    where
        Self: Sized,
    {
        render_ctx
    }

    fn duration_since(&self, earlier: &Self) -> std::time::Duration {
        std::time::Duration::from_nanos((self.0 - earlier.0).try_into().unwrap())
    }

    fn add_duration(self, duration: std::time::Duration) -> Self {
        Self(self.0 + duration.as_nanos())
    }

    fn query_presentation_statistics(
        surface: &wgpu::Surface,
        _window: &winit::window::Window,
        fallback: Self,
    ) -> Self
    where
        Self: Sized,
    {
        // let presentation_stats = surface.query_presentation_statistics();

        // match presentation_stats.last() {
        //     Some(stats) => stats.presentation_start,

        //     None => {
        //         log::warn!("unable to retrieve presentation stats");

        //         fallback
        //     }
        // }

        fallback
    }
}

static STATIC_NON_CONTEXT: () = ();

impl InstantLike for std::time::Instant {
    type Context = ();

    fn now(_: &Self::Context) -> Self {
        Self::now()
    }

    fn context_from(render_ctx: &RenderingContext) -> &Self::Context
    where
        Self: Sized,
    {
        &STATIC_NON_CONTEXT
    }

    fn duration_since(&self, earlier: &Self) -> std::time::Duration {
        self.duration_since(*earlier)
    }

    fn add_duration(self, duration: std::time::Duration) -> Self {
        self + duration
    }
}

impl InstantLike for std::time::Duration {
    type Context = ();

    fn now(_: &Self::Context) -> Self {
        // TODO: non-osx impl
        Self::from_secs_f64(unsafe { NSProcessInfo::processInfo().systemUptime() })
    }

    fn context_from(render_ctx: &RenderingContext) -> &Self::Context
    where
        Self: Sized,
    {
        &STATIC_NON_CONTEXT
    }

    fn duration_since(&self, earlier: &Self) -> std::time::Duration {
        *self - *earlier
    }

    fn add_duration(self, duration: std::time::Duration) -> Self {
        self + duration
    }

    fn query_presentation_statistics(
        _surface: &wgpu::Surface,
        window: &winit::window::Window,
        fallback: Self,
    ) -> Self
    where
        Self: Sized,
    {
        get_window_last_screen_draw_time(window).unwrap()
    }
}

pub struct ManagedFramepacer<I: InstantLike = FramepacerInstant> {
    // time in seconds
    last_30: ConstGenericRingBuffer<f64, FRAMEPACER_NUM_SAMPLES>,
    i: usize,

    worst_frametime_secs: f64,

    deadline: Option<I>,

    desired_frame_time: Option<std::time::Duration>,

    last_presentation_start: Option<I>,
    last_presentation_interval: Option<std::time::Duration>,
}

impl<I: InstantLike> Default for ManagedFramepacer<I> {
    fn default() -> Self {
        Self {
            last_30: Default::default(),
            i: Default::default(),
            worst_frametime_secs: Default::default(),
            deadline: Default::default(),
            desired_frame_time: Default::default(),
            last_presentation_start: Default::default(),
            last_presentation_interval: Default::default(),
        }
    }
}

const DEFAULT_FRAME_TIME_SECS: f64 = 1. / 60.;

const DEVIATION_BUFFER_MICROS: u64 = 30;

pub trait Framepacer<I: InstantLike = FramepacerInstant> {
    fn new() -> Self
    where
        Self: Sized + Default,
    {
        Default::default()
    }

    fn start_window(
        &mut self,
        presentation_start: I,
        screen_refresh_time: Option<std::time::Duration>,
    );

    fn check_missed_deadline(&mut self, now: I, render_time: Option<std::time::Duration>) -> bool;

    fn get_deadline(&self) -> Option<I>;

    fn should_render(&mut self, start_time: I) -> (bool, I);

    fn push_frametime(&mut self, duration: crate::time::Duration);

    fn desired_frame_time(&self) -> Option<std::time::Duration>;

    fn desired_frame_instant(&self) -> Option<wgpu::PresentationTimestamp>
    where
        I: Copy,
    {
        None
    }

    fn sync_to_fps(&self, fps: f32) {}
}

use icrate::{
    AppKit::NSScreen,
    CoreAnimation::{CADisplayLink, CAFrameRateRangeMake, CFTimeInterval},
    Foundation::NSProcessInfo,
};
use objc2::rc::Id;
use std::os::raw::c_int;

use icrate::Foundation::{
    NSCopying, NSObject, NSObjectProtocol, NSRunLoop, NSRunLoopCommonModes, NSZone,
};
use objc2::declare::{Ivar, IvarBool, IvarDrop, IvarEncode};
use objc2::{
    declare_class, extern_protocol, msg_send, msg_send_id, mutability, sel, ClassType, ProtocolType,
};

declare_class!(
    struct CADisplayLinkPollable {
        target_timestamp: IvarEncode<CFTimeInterval, "_target_timestamp">,
        timestamp: IvarEncode<CFTimeInterval, "_timestamp">,
        is_ready: IvarBool<"_is_ready">,
        ca_display_link: IvarDrop<Id<CADisplayLink>, "_ca_display_link">,
    }

    mod ivar;

    unsafe impl ClassType for CADisplayLinkPollable {
        type Super = NSObject;
        type Mutability = mutability::Mutable;
        const NAME: &'static str = "CADisplayLinkPollable";
    }

    unsafe impl CADisplayLinkPollable {
        #[method(init)]
        fn init(this: &mut Self) -> Option<&mut Self> {
            let this: Option<&mut Self> = unsafe { msg_send![super(this), init] };

            this.map(|this| {
                let ca_display_link =
                    unsafe { CADisplayLink::displayLinkWithTarget_selector(this, sel!(step:)) };

                unsafe {
                    ca_display_link
                        .addToRunLoop_forMode(&NSRunLoop::currentRunLoop(), NSRunLoopCommonModes);
                }

                Ivar::write(&mut this.ca_display_link, ca_display_link);
                *this.is_ready = false;

                // All the instance variables have been initialized; our
                // initializer is sound
                this
            })
        }

        #[method(step:)]
        fn step(this: &mut Self, ca_display_link: &mut CADisplayLink) {
            *this.target_timestamp = unsafe { ca_display_link.targetTimestamp() };
            *this.timestamp = unsafe { ca_display_link.timestamp() };

            *this.is_ready = true;
        }

        #[method(isReady)]
        fn __get_is_ready(&self) -> bool {
            *self.is_ready
        }

        #[method(targetTimestamp)]
        fn __get_target_timestamp(&mut self) -> CFTimeInterval {
            *self.is_ready = false;
            *self.target_timestamp
        }

        #[method(timestamp)]
        fn __get_timestamp(&mut self) -> CFTimeInterval {
            *self.timestamp
        }

        #[method_id(caDisplayLink)]
        fn __get_ca_display_link(&self) -> Id<CADisplayLink> {
            self.ca_display_link.clone()
        }
    }
);

impl CADisplayLinkPollable {
    pub fn new() -> Id<Self> {
        unsafe { msg_send_id![Self::alloc(), init] }
    }

    pub fn get_is_ready(&self) -> bool {
        unsafe { msg_send![self, isReady] }
    }

    pub fn get_target_timestamp(&self) -> CFTimeInterval {
        unsafe { msg_send![self, targetTimestamp] }
    }

    pub fn get_timestamp(&self) -> CFTimeInterval {
        unsafe { msg_send![self, timestamp] }
    }

    pub fn get_ca_display_link(&self) -> Id<CADisplayLink> {
        unsafe { msg_send_id![self, caDisplayLink] }
    }
}

pub struct CADisplayLinkFramepacer {
    ca_display_link_pollable: Id<CADisplayLinkPollable>,
    managed_framepacer: ManagedFramepacer<std::time::Duration>,

    next_deadline: Option<std::time::Duration>,
    last_deadline: Option<std::time::Duration>,

    desired_frametime: Option<std::time::Duration>,

    needs_kickstart: bool,
}

impl CADisplayLinkFramepacer {
    pub fn new() -> Self {
        Self {
            ca_display_link_pollable: CADisplayLinkPollable::new(),
            managed_framepacer: Default::default(),

            next_deadline: Default::default(),
            last_deadline: Default::default(),

            desired_frametime: Default::default(),

            needs_kickstart: true,
        }
    }
}

impl Framepacer<std::time::Duration> for CADisplayLinkFramepacer {
    fn start_window(
        &mut self,
        presentation_start: std::time::Duration,
        screen_refresh_time: Option<std::time::Duration>,
    ) {
        self.managed_framepacer
            .start_window(presentation_start, screen_refresh_time);
    }

    fn check_missed_deadline(
        &mut self,
        now: std::time::Duration,
        render_time: Option<std::time::Duration>,
    ) -> bool {
        self.managed_framepacer
            .check_missed_deadline(now, render_time)
    }

    fn get_deadline(&self) -> Option<std::time::Duration> {
        self.next_deadline
    }

    fn should_render(&mut self, start_time: std::time::Duration) -> (bool, std::time::Duration) {
        let new_deadline = if self.ca_display_link_pollable.get_is_ready() {
            Some(std::time::Duration::from_secs_f64(
                self.ca_display_link_pollable.get_target_timestamp(),
            ))
        } else {
            None
        };

        if let Some(deadline) = new_deadline {
            // if self.next_deadline.is_some() {
            //     log::warn!("deadline override!");
            // }

            let old_deadline = self.managed_framepacer.deadline.replace(deadline);

            // if let Some(old_deadline) = old_deadline {
            //     println!(
            //         "managed deadline mismatch: {:?}",
            //         duration_dist(old_deadline, deadline)
            //     )
            // }

            // self.last_deadline = self.next_deadline.take();

            self.next_deadline = deadline.into();
        }

        if let Some(next_deadline) = self.next_deadline {
            let result = self.managed_framepacer.should_render(start_time);

            if result.0 {
                self.desired_frametime = (next_deadline
                    - std::time::Duration::from_secs_f64(
                        self.ca_display_link_pollable.get_timestamp(),
                    ))
                .into();

                // if let Some(last_deadline) = self.last_deadline {
                //     self.desired_frametime = (next_deadline - last_deadline).into();
                // }

                self.next_deadline = None;
            }

            result
        } else {
            (false, start_time)
        }
    }

    fn push_frametime(&mut self, duration: crate::time::Duration) {
        self.managed_framepacer.push_frametime(duration)
    }

    fn desired_frame_time(&self) -> Option<std::time::Duration> {
        self.desired_frametime

        // self.desired_frametime.and_then(|duration| {
        //     if let Some(screen_refresh) = self.managed_framepacer.desired_frame_time {
        //         if duration > screen_refresh + std::time::Duration::from_micros(500) {
        //             None
        //         } else {
        //             Some(duration)
        //         }
        //     } else {
        //         None
        //     }
        // })
    }

    fn desired_frame_instant(&self) -> Option<wgpu::PresentationTimestamp>
    where
        std::time::Duration: Copy,
    {
        self.get_deadline()
            .map(|deadline| wgpu::PresentationTimestamp(deadline.as_nanos()))
    }

    fn sync_to_fps(&self, fps: f32) {
        unsafe {
            self.ca_display_link_pollable
                .get_ca_display_link()
                .setPreferredFrameRateRange(CAFrameRateRangeMake(fps, fps, fps))
        }
    }
}

pub struct NoopFramepacer {
    screen_refresh_time: Option<std::time::Duration>,
}

impl Default for NoopFramepacer {
    fn default() -> Self {
        Self {
            screen_refresh_time: Default::default(),
        }
    }
}

impl<I: InstantLike> Framepacer<I> for NoopFramepacer {
    fn start_window(
        &mut self,
        presentation_start: I,
        screen_refresh_time: Option<std::time::Duration>,
    ) {
        self.screen_refresh_time = screen_refresh_time
    }

    fn check_missed_deadline(&mut self, now: I, render_time: Option<std::time::Duration>) -> bool {
        false
    }

    fn get_deadline(&self) -> Option<I> {
        None
    }

    fn should_render(&mut self, start_time: I) -> (bool, I) {
        (true, start_time)
    }

    fn push_frametime(&mut self, duration: crate::time::Duration) {}

    fn desired_frame_time(&self) -> Option<std::time::Duration> {
        self.screen_refresh_time
    }
}

impl<I: InstantLike + Copy + std::fmt::Debug> Framepacer<I> for ManagedFramepacer<I> {
    fn new() -> Self {
        Default::default()
    }

    fn start_window(
        &mut self,
        presentation_start: I,
        screen_refresh_time: Option<std::time::Duration>,
    ) {
        if let Some(deadline) = self.deadline {
            // if presentation_start < deadline {
            //     println!(
            //         "input lag: {:?}",
            //         deadline.duration_since(&presentation_start)
            //     );
            // }
        }

        if let Some(last_presentation_start) = self.last_presentation_start {
            if presentation_start > last_presentation_start {
                let last_presentation_interval =
                    presentation_start.duration_since(&last_presentation_start);

                self.last_presentation_interval = last_presentation_interval.into();
            }
        }

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

        let desired_frame_time = screen_refresh_time
            .unwrap_or_else(|| std::time::Duration::from_secs_f64(DEFAULT_FRAME_TIME_SECS));

        // let desired_frame_time = Option::zip(self.last_presentation_interval, screen_refresh_time)
        //     .map(|(last_presentation_interval, screen_refresh_time)| {
        //         let diff = last_presentation_interval
        //             .saturating_sub(screen_refresh_time)
        //             .max(screen_refresh_time.saturating_sub(last_presentation_interval));

        //         if diff > std::time::Duration::from_micros(400) {
        //             screen_refresh_time
        //         } else {
        //             last_presentation_interval
        //         }
        //     })
        //     .or(screen_refresh_time)
        //     .unwrap_or_else(|| std::time::Duration::from_secs_f64(DEFAULT_FRAME_TIME_SECS));

        self.desired_frame_time = desired_frame_time.into();
        self.last_presentation_start = presentation_start.into();

        self.deadline = presentation_start.add_duration(desired_frame_time).into();
    }

    fn check_missed_deadline(&mut self, now: I, render_time: Option<std::time::Duration>) -> bool {
        if let Some(deadline) = self.deadline {
            let missed = now > deadline;

            if missed {
                // log::debug!("missed deadline by {:?}!", now.duration_since(&deadline));

                let predicted_frametime =
                    crate::time::Duration::from_secs_f64(self.worst_frametime_secs)
                        + crate::time::Duration::from_micros(DEVIATION_BUFFER_MICROS);

                if let Some(render_time) = render_time {
                    // log::debug!(
                    //     "\trender time: {:?}, anticipated: {:?}",
                    //     render_time,
                    //     predicted_frametime
                    // );
                }
            }

            missed
        } else {
            false
        }
    }

    fn get_deadline(&self) -> Option<I> {
        self.deadline
    }

    fn should_render(&mut self, start_time: I) -> (bool, I) {
        let should_render = match self.deadline {
            Some(deadline) => {
                let predicted_finish_time = start_time.add_duration(
                    crate::time::Duration::from_secs_f64(self.worst_frametime_secs)
                        + crate::time::Duration::from_micros(DEVIATION_BUFFER_MICROS),
                );

                // TODO: add buffer here for input/parsing time...
                predicted_finish_time >= deadline
            }

            None => true,
        };

        (should_render, start_time)
    }

    //  fn next_deadline(&mut self, from: crate::time::Instant) -> crate::time::Instant {}

    fn push_frametime(&mut self, duration: crate::time::Duration) {
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

    fn desired_frame_time(&self) -> Option<std::time::Duration> {
        self.desired_frame_time
    }
}

fn duration_dist(d1: std::time::Duration, d2: std::time::Duration) -> std::time::Duration {
    d1.saturating_sub(d2).max(d2.saturating_sub(d1))
}
