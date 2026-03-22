use crate::capture::AudioInput;
use crate::config::{CaptureConfig, PlaybackConfig, StreamParams};
use crate::error::Result;
use crate::playback::AudioOutput;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod unsupported;

pub fn open_input(config: &CaptureConfig, stream: &StreamParams) -> Result<Box<dyn AudioInput>> {
    #[cfg(target_os = "macos")]
    {
        return macos::open_input(config, stream);
    }
    #[cfg(target_os = "windows")]
    {
        return windows::open_input(config, stream);
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        return unsupported::open_input(config, stream);
    }
}

pub fn open_output(config: &PlaybackConfig, stream: &StreamParams) -> Result<Box<dyn AudioOutput>> {
    #[cfg(target_os = "macos")]
    {
        return macos::open_output(config, stream);
    }
    #[cfg(target_os = "windows")]
    {
        return windows::open_output(config, stream);
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        return unsupported::open_output(config, stream);
    }
}
