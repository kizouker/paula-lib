use std::fmt;

pub trait AudioBackend {
    fn available(&self) -> bool;
    fn error(&self) -> Option<&str>;
}

#[derive(Clone, Debug, Default)]
pub struct SimulatedAudio {
    pub available: bool,
    pub error: Option<String>,
}

impl SimulatedAudio {
    pub fn available() -> Self {
        Self { available: true, error: None }
    }

    pub fn unavailable(error: impl Into<String>) -> Self {
        Self { available: false, error: Some(error.into()) }
    }
}

impl AudioBackend for SimulatedAudio {
    fn available(&self) -> bool { self.available }
    fn error(&self) -> Option<&str> { self.error.as_deref() }
}

pub fn fallback_notice(error: impl fmt::Display) -> String {
    format!("no audio device ({error}); lint and save still work")
}

pub fn probe_device() -> Result<u32, String> {
    paula::output::device_rate()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simulated_audio_reports_availability() {
        let audio = SimulatedAudio::unavailable("test");
        assert!(!audio.available());
        assert_eq!(audio.error(), Some("test"));
        assert_eq!(fallback_notice("test"), "no audio device (test); lint and save still work");
    }
}
