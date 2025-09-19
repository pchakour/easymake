use core::fmt;


#[derive(Clone)]
pub struct LoopError {
    pub path: String,
}

impl fmt::Display for LoopError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Target loop found {}", self.path)
    }
}

impl fmt::Debug for LoopError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "LoopError path: {}", self.path)
    }
}