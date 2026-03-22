use crate::config::{CaptureConfig, StreamParams};
use crate::error::Result;
use crate::platform;
use std::time::Duration;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaptureStatus {
    Ok,
    Timeout,
}

pub trait AudioInput: Send {
    fn read_frame(&mut self, frame: &mut [f32], timeout: Duration) -> Result<CaptureStatus>;
}

pub fn open_input(config: &CaptureConfig, stream: &StreamParams) -> Result<Box<dyn AudioInput>> {
    platform::open_input(config, stream)
}
