use keyframe::EasingFunction;

use crate::scene::ctx::SceneContext;

pub struct Transition {
    easing_func: Option<Box<dyn EasingFunction>>,
    time: f32,
    state: bool,
    fac: f32,
}

impl Transition {
    pub fn new(time: f32) -> Self {
        Self {
            easing_func: None,
            state: false,
            time,
            fac: 0.,
        }
    }

    pub fn update(&mut self, ctx: &mut SceneContext) {
        let dt = ctx.input().stable_dt.min(0.1);
        let fac = dt / self.time;

        match self.state {
            true => self.fac = (self.fac + fac).min(1.),
            false => self.fac = (self.fac - fac).max(0.),
        }
    }

    pub fn set_state(&mut self, state: bool) {
        self.state = state;
    }

    pub fn set_ease_func(mut self, func: impl EasingFunction + 'static) -> Self {
        self.easing_func = Some(Box::new(func));
        self
    }

    pub fn state(&self) -> bool {
        self.state
    }

    pub fn fac(&self) -> f32 {
        match &self.easing_func {
            None => self.fac,
            Some(ease) => ease.y(self.fac as f64) as f32,
        }
    }
}
