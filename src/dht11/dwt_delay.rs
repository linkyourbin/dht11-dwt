use cortex_m::peripheral::DWT;

/// DWT-based delay provider that works with any CPU frequency
pub struct DwtDelay {
    cycles_per_us: u32,
}

impl DwtDelay {
    /// Create a new DWT delay with the given CPU frequency in Hz
    pub fn new(cpu_freq_hz: u32) -> Self {
        Self {
            cycles_per_us: cpu_freq_hz / 1_000_000,
        }
    }

    /// Delay for approximately 1 microsecond
    #[inline(always)]
    pub fn delay_1us(&self) {
        let start = DWT::cycle_count();

        loop {
            let current = DWT::cycle_count();
            let elapsed = current.wrapping_sub(start);
            if elapsed >= self.cycles_per_us {
                break;
            }
        }
    }
}
