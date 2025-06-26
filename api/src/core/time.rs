use axerrno::{LinuxError, LinuxResult};
use axhal::time::NANOS_PER_SEC;
use core::time::Duration;

/// Nanosecond-precision timeout specification, equivalent to C's `struct timespec`.
#[repr(C)]
/// Time in seconds and nanoseconds
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(C)]
pub struct TimeSpec {
    /// seconds
    pub seconds: i64,
    /// nanoseconds in range [0, 999_999_999]
    pub nanoseconds: i64,
}

impl TimeSpec {
    pub fn validate(&self) -> LinuxResult<&Self> {
        if self.nanoseconds < 0 || self.nanoseconds >= NANOS_PER_SEC as _ {
            return Err(LinuxError::EINVAL);
        }
        if self.seconds < 0 && self.nanoseconds > 0 {
            return Err(LinuxError::EINVAL);
        }
        Ok(self)
    }

    pub fn to_duration(&self) -> LinuxResult<Duration> {
        self.validate()?;
        Ok(Duration::new(self.seconds as u64, self.nanoseconds as u32))
    }
}

// 注意：尝试转换不合法的时间会返回默认值
impl From<TimeSpec> for Duration {
    fn from(ts: TimeSpec) -> Self {
        if ts.seconds < 0 || ts.nanoseconds < 0 || ts.nanoseconds >= NANOS_PER_SEC as _ {
            return Duration::default();
        }
        Duration::new(ts.seconds as u64, ts.nanoseconds as u32)
    }
}

impl From<Duration> for TimeSpec {
    fn from(duration: Duration) -> Self {
        TimeSpec {
            seconds: duration.as_secs() as _,
            nanoseconds: duration.subsec_nanos() as _,
        }
    }
}
