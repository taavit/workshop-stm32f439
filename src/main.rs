#![no_std]
#![no_main]

use cortex_m_rt::entry;
use defmt::info;
use defmt_rtt as _;
use panic_probe as _;
use stm32f4xx_hal::gpio::GpioExt;
use stm32f4xx_hal::pac::Peripherals;
use stm32f4xx_hal::prelude::*;

use stm32_morse::{
    game::{Game, GameResult},
    rng::RandomFromNoise,
    MemoryGame,
};
use stm32f4xx_hal::{
    adc::{config::AdcConfig, Adc},
    pac::ADC1,
};

mod game;

#[entry]
fn main() -> ! {
    let dp = Peripherals::take().unwrap();
    let cp = cortex_m::peripheral::Peripherals::take().unwrap();

    let gpiob = dp.GPIOB.split();
    let gpioc = dp.GPIOC.split();

    let rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.sysclk(48.MHz()).freeze();
    let delay = cp.SYST.delay(&clocks);
    let timer = dp.TIM2.counter_ms(&clocks);

    let adc: Adc<ADC1> = Adc::adc1(dp.ADC1, false, AdcConfig::default());
    let analog_pin = gpiob.pb0.into_analog();
    let rng = RandomFromNoise::new(adc, analog_pin);

    let led = gpiob.pb7.into_push_pull_output();
    let button = gpioc.pc13.into_pull_down_input();

    let mut game = MemoryGame::new(led, button, delay, rng, timer);
    loop {
        match game.play() {
            GameResult::Correct => {
                let level = game.advance();
                info!("Correct! Next level: {}", level);
            }
            GameResult::Incorrect => {
                info!("Incorrect! Starting new game.");
                game.new_game();
            }
        }
    }
}
