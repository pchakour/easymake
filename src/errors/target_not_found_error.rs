use core::fmt;
use std::fmt::Display;


#[derive(Clone)]
pub struct TargetNotFoundError {
    pub target: String,
}

impl Display for TargetNotFoundError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Target not found {}", self.target)
    }
}

impl fmt::Debug for TargetNotFoundError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TargetNotFoundError target: {}", self.target)
    }
}