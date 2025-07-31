//! CPU cycle measurement and profiling for Bract compiler
//! 
//! Provides precise timing using hardware performance counters and RDTSC

use std::arch::x86_64::_rdtsc;
use std::time::{Duration, Instant};

/// High-precision profiler using CPU cycles
#[derive(Debug)]
pub struct CycleProfiler {
    /// Start cycle count
    start_cycles: u64,
    /// Start wall time
    start_time: Instant,
    /// CPU frequency estimate (cycles per second)
    cpu_freq_hz: Option<u64>,
}

impl CycleProfiler {
    /// Create a new cycle profiler
    pub fn new() -> Self {
        Self {
            start_cycles: 0,
            start_time: Instant::now(),
            cpu_freq_hz: None,
        }
    }

    /// Start profiling - record current CPU cycle count
    pub fn start(&mut self) {
        unsafe {
            self.start_cycles = _rdtsc();
        }
        self.start_time = Instant::now();
    }

    /// Stop profiling and return results
    pub fn stop(&self) -> ProfilingResult {
        let end_time = Instant::now();
        let end_cycles = unsafe { _rdtsc() };
        
        let wall_time = end_time.duration_since(self.start_time);
        let cycle_count = end_cycles.saturating_sub(self.start_cycles);
        
        // estimate cpu frequency from this measurement
        let estimated_freq = if wall_time.as_nanos() > 0 {
            Some((cycle_count as f64 / wall_time.as_secs_f64()) as u64)
        } else {
            None
        };

        ProfilingResult {
            wall_time,
            cpu_cycles: cycle_count,
            estimated_cpu_freq: estimated_freq,
        }
    }

    /// Quick measurement of a function
    pub fn measure<F, R>(mut self, f: F) -> (R, ProfilingResult)
    where
        F: FnOnce() -> R,
    {
        self.start();
        let result = f();
        let profile = self.stop();
        (result, profile)
    }
}

/// Results from cycle profiling
#[derive(Debug, Clone)]
pub struct ProfilingResult {
    /// Wall clock time elapsed
    pub wall_time: Duration,
    /// CPU cycles elapsed
    pub cpu_cycles: u64,
    /// Estimated CPU frequency (Hz)
    pub estimated_cpu_freq: Option<u64>,
}

impl ProfilingResult {
    /// Get cycles per microsecond (useful for comparison)
    pub fn cycles_per_microsecond(&self) -> f64 {
        if self.wall_time.as_micros() > 0 {
            self.cpu_cycles as f64 / self.wall_time.as_micros() as f64
        } else {
            0.0
        }
    }

    /// Get estimated CPU frequency in GHz
    pub fn cpu_freq_ghz(&self) -> Option<f64> {
        self.estimated_cpu_freq.map(|freq| freq as f64 / 1_000_000_000.0)
    }

    /// Display human-readable timing info
    pub fn display(&self) -> String {
        let mut output = String::new();
        
        output.push_str(&format!("⏱️  Wall time: {:.3}ms\n", self.wall_time.as_secs_f64() * 1000.0));
        output.push_str(&format!("🔄 CPU cycles: {}\n", self.cpu_cycles));
        output.push_str(&format!("📊 Cycles/μs: {:.1}\n", self.cycles_per_microsecond()));
        
        if let Some(freq_ghz) = self.cpu_freq_ghz() {
            output.push_str(&format!("⚡ CPU freq: {:.2} GHz\n", freq_ghz));
        }

        output
    }
}

/// Macro for quick profiling
#[macro_export]
macro_rules! profile_cycles {
    ($expr:expr) => {{
        let profiler = $crate::profiling::CycleProfiler::new();
        profiler.measure(|| $expr)
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_cycle_profiler() {
        let (result, profile) = CycleProfiler::new().measure(|| {
            // small computation to measure
            (0..1000).sum::<i32>()
        });

        assert_eq!(result, 499500); // sum of 0..1000
        assert!(profile.cpu_cycles > 0);
        assert!(profile.wall_time.as_nanos() > 0);
        println!("{}", profile.display());
    }

    #[test]
    fn test_sleep_measurement() {
        let (_, profile) = CycleProfiler::new().measure(|| {
            thread::sleep(Duration::from_millis(1));
        });

        assert!(profile.wall_time >= Duration::from_millis(1));
        assert!(profile.cpu_cycles > 0);
        println!("Sleep measurement:\n{}", profile.display());
    }
}