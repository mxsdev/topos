use super::boundary::{Boundary, EmptyBoundary};

pub trait Element {
    fn takes_focus(&self) -> bool {
        true
    }

    fn boundary(&self) -> &impl Boundary {
        &EmptyBoundary
    }
}
