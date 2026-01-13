#![no_std]
#![no_main]
#[allow(unused_imports)]

use defmt::info;
use embassy_executor::Spawner;
use embassy_stm32::{gpio::{Flex, Level, Output, Speed}, time::Hertz};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};
use embassy_stm32::Config;

mod dht11;
use dht11::Dht11;

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    let mut config = Config::default();
    {
        use embassy_stm32::rcc::*;
        config.rcc.hse = Some(Hse {
            freq: Hertz(8_000_000),
            mode: HseMode::Oscillator,
        });
        config.rcc.pll_src = PllSource::HSE;
        config.rcc.pll = Some(Pll {
            prediv: PllPreDiv::DIV4,
            mul: PllMul::MUL168,
            divp: Some(PllPDiv::DIV2), // 8mhz / 4 * 168 / 2 = 168Mhz.
            divq: Some(PllQDiv::DIV7), // 8mhz / 4 * 168 / 7 = 48Mhz.
            divr:Some(PllRDiv::DIV2),  // 8mhz / 4 * 168 / 2 = 168Mhz.
        });
        config.rcc.ahb_pre = AHBPrescaler::DIV1;
        config.rcc.apb1_pre = APBPrescaler::DIV4;
        config.rcc.apb2_pre = APBPrescaler::DIV2;
        config.rcc.sys = Sysclk::PLL1_P;
        config.rcc.mux.clk48sel = mux::Clk48sel::PLL1_Q;
    }
    let p = embassy_stm32::init(config);

    let mut led = Output::new(p.PB13, Level::High, Speed::VeryHigh);

    // Initialize DHT11 on PA8
    let dht_pin = Flex::new(p.PA8);
    let mut dht11 = Dht11::new(dht_pin);

    info!("DHT11 sensor initialized on PA8");

    // Wait for DHT11 to stabilize after power-on (at least 1 second)
    Timer::after_secs(2).await;

    loop {
        led.toggle();

        // Read DHT11 sensor
        match dht11.read().await {
            Ok(reading) => {
                info!("Temperature: {}°C, Humidity: {}%",
                      reading.temperature, reading.humidity);
            }
            Err(e) => {
                info!("DHT11 read error: {:?}", e);
            }
        }

        // DHT11 needs at least 2 seconds between readings
        Timer::after_secs(2).await;
    }
}
