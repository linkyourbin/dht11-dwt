use embassy_stm32::gpio::{Flex, Pull, Speed};
use embassy_time::Timer;
use super::dwt_delay::DwtDelay;

#[derive(Debug, Clone, Copy, defmt::Format)]
pub struct DhtReading {
    pub temperature: f32,
    pub humidity: f32,
}

#[derive(Debug, defmt::Format)]
pub enum DhtError {
    Timeout,
    ChecksumError,
}

pub struct Dht11<'d> {
    pin: Flex<'d>,
    delay: DwtDelay,
}

impl<'d> Dht11<'d> {
    /// Create a new DHT11 driver
    ///
    /// # Arguments
    /// * `pin` - Flex GPIO pin for DHT11 data line
    /// * `cpu_freq_hz` - CPU frequency in Hz (e.g., 168_000_000 for 168MHz)
    pub fn new(mut pin: Flex<'d>, cpu_freq_hz: u32) -> Self {
        // Initialize pin as output high
        pin.set_as_output(Speed::VeryHigh);
        pin.set_high();
        Self {
            pin,
            delay: DwtDelay::new(cpu_freq_hz),
        }
    }

    pub async fn read(&mut self) -> Result<DhtReading, DhtError> {
        // Send start signal: pull low for at least 18ms
        self.pin.set_as_output(Speed::VeryHigh);
        self.pin.set_low();
        Timer::after_millis(20).await;

        // Pull high and wait 20-40us
        self.pin.set_high();
        Timer::after_micros(40).await;

        // Switch to input mode with pull-up
        self.pin.set_as_input(Pull::Up);

        // Wait for DHT11 to pull low (response signal)
        self.wait_for_low(100)?;

        // Wait for DHT11 to pull high
        self.wait_for_high(100)?;

        // Wait for DHT11 to pull low (start of data)
        self.wait_for_low(100)?;

        // Read 40 bits of data
        let mut data = [0u8; 5];
        let mut pulse_widths = [0u32; 8]; // Debug: store first 8 pulse widths
        let mut pulse_idx = 0;

        for byte in data.iter_mut() {
            for bit in (0..8).rev() {
                // Wait for start of bit (high)
                self.wait_for_high(100)?;

                // Measure high pulse duration
                // '0' = ~28us high, '1' = ~70us high
                let mut high_time = 0u32;
                while self.pin.is_high() && high_time < 200 {
                    self.delay.delay_1us();
                    high_time += 1;
                }

                // Store first 8 pulse widths for debugging
                if pulse_idx < 8 {
                    pulse_widths[pulse_idx] = high_time;
                    pulse_idx += 1;
                }

                // Adjust threshold based on measured values:
                // Short pulse (0): ~8-12us, Long pulse (1): ~30-34us
                // Use 20us as threshold
                if high_time > 20 {
                    *byte |= 1 << bit;
                }

                // Wait for end of bit (low)
                self.wait_for_low(100)?;
            }
        }

        // Pull pin high to finish
        self.pin.set_as_output(Speed::VeryHigh);
        self.pin.set_high();

        // Debug: print first 8 pulse widths
        defmt::debug!("First 8 pulse widths (us): [{}, {}, {}, {}, {}, {}, {}, {}]",
                      pulse_widths[0], pulse_widths[1], pulse_widths[2], pulse_widths[3],
                      pulse_widths[4], pulse_widths[5], pulse_widths[6], pulse_widths[7]);

        // Debug: print raw data
        defmt::debug!("Raw data: [{:02x}, {:02x}, {:02x}, {:02x}, {:02x}]",
                      data[0], data[1], data[2], data[3], data[4]);

        // Verify checksum
        let checksum = data[0]
            .wrapping_add(data[1])
            .wrapping_add(data[2])
            .wrapping_add(data[3]);

        defmt::debug!("Calculated checksum: {:02x}, Received: {:02x}", checksum, data[4]);

        if checksum != data[4] {
            return Err(DhtError::ChecksumError);
        }

        // Parse data (DHT11 only uses integer part)
        let humidity = data[0] as f32 + (data[1] as f32) * 0.1;
        let temperature = data[2] as f32 + (data[3] as f32) * 0.1;

        Ok(DhtReading {
            temperature,
            humidity,
        })
    }

    fn wait_for_low(&mut self, timeout_us: u32) -> Result<(), DhtError> {
        let mut count = 0;
        while self.pin.is_high() {
            count += 1;
            if count > timeout_us {
                return Err(DhtError::Timeout);
            }
            self.delay.delay_1us();
        }
        Ok(())
    }

    fn wait_for_high(&mut self, timeout_us: u32) -> Result<(), DhtError> {
        let mut count = 0;
        while self.pin.is_low() {
            count += 1;
            if count > timeout_us {
                return Err(DhtError::Timeout);
            }
            self.delay.delay_1us();
        }
        Ok(())
    }
}
