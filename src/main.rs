#![no_std]
#![no_main]

use cortex_m_rt::entry;
use defmt::{debug, info, Format};
use embedded_hal::{
    delay::DelayNs,
    digital::{InputPin, OutputPin},
};
use embedded_hal_02::adc::Channel;
use fugit::{Duration, ExtU32};
use heapless::{String, Vec};
use panic_probe as _;
use rand_core::RngCore;

use defmt_rtt as _;

use stm32f4xx_hal::{
    adc::{
        config::{AdcConfig, SampleTime},
        Adc,
    },
    pac::{self, ADC1},
    prelude::*,
};

struct RandomFromNoise<PIN: Channel<ADC1>> {
    adc: Adc<ADC1>,
    pin: PIN,
}

impl<PIN: Channel<ADC1>> RandomFromNoise<PIN> {
    pub fn new(adc: Adc<ADC1>, pin: PIN) -> Self {
        Self { adc, pin }
    }
}

impl<PIN: Channel<ADC1, ID = u8>> RngCore for RandomFromNoise<PIN> {
    fn fill_bytes(&mut self, _: &mut [u8]) {
        unimplemented!();
    }
    fn next_u32(&mut self) -> u32 {
        let mut res: u32 = 0;
        res += self.adc.convert(&self.pin, SampleTime::Cycles_3) as u32;
        res <<= 4;
        res += self.adc.convert(&self.pin, SampleTime::Cycles_3) as u32;
        res <<= 4;
        res += self.adc.convert(&self.pin, SampleTime::Cycles_3) as u32;
        res <<= 4;
        res += self.adc.convert(&self.pin, SampleTime::Cycles_3) as u32;
        res <<= 4;
        res += self.adc.convert(&self.pin, SampleTime::Cycles_3) as u32;
        res <<= 4;
        res += self.adc.convert(&self.pin, SampleTime::Cycles_3) as u32;
        res <<= 4;
        res += self.adc.convert(&self.pin, SampleTime::Cycles_3) as u32;
        res <<= 4;
        res += self.adc.convert(&self.pin, SampleTime::Cycles_3) as u32;

        res
    }
    fn next_u64(&mut self) -> u64 {
        unimplemented!();
    }
    fn try_fill_bytes(&mut self, _: &mut [u8]) -> Result<(), rand_core::Error> {
        unimplemented!();
    }
}

/// Represents signal duration
#[derive(Debug, PartialEq, Clone, Copy)]
enum Signal {
    Short,
    Long,
}

impl Format for Signal {
    fn format(&self, fmt: defmt::Formatter) {
        match self {
            Signal::Short => defmt::write!(fmt, "."),
            Signal::Long => defmt::write!(fmt, "-"),
        }
    }
}

impl Signal {
    pub fn from_duration<const TIMER_HZ: u32>(value: Duration<u32, 1, TIMER_HZ>) -> Option<Self> {
        if value < Duration::<u32, 1, TIMER_HZ>::millis(10) {
            return None;
        }

        if value <= Duration::<u32, 1, TIMER_HZ>::millis(500) {
            return Some(Signal::Short);
        }

        Some(Signal::Long)
    }
}

impl From<u32> for Signal {
    fn from(value: u32) -> Self {
        match value % 2 {
            0 => Signal::Short,
            1 => Signal::Long,
            _ => unreachable!(),
        }
    }
}

impl From<&Signal> for char {
    fn from(val: &Signal) -> char {
        match val {
            Signal::Short => '.',
            Signal::Long => '-',
        }
    }
}

#[derive(Debug, PartialEq)]
enum GameResult {
    Correct,
    Incorrect,
}

impl From<bool> for GameResult {
    fn from(value: bool) -> Self {
        match value {
            true => GameResult::Correct,
            false => GameResult::Incorrect,
        }
    }
}

struct MemoryGame<
    const TIMER_HZ: u32,
    Random: RngCore,
    Delay: DelayNs,
    Led: OutputPin,
    Button: InputPin,
    Timer: fugit_timer::Timer<TIMER_HZ>,
> {
    led: Led,
    button: Button,
    delay: Delay,
    random: Random,
    timer: Timer,

    level: u8,
    state: Vec<Signal, 64>,
}

impl<
        const TIMER_HZ: u32,
        Random: RngCore,
        Delay: DelayNs,
        Led: OutputPin,
        Button: InputPin,
        Timer: fugit_timer::Timer<TIMER_HZ>,
    > MemoryGame<TIMER_HZ, Random, Delay, Led, Button, Timer>
{
    pub fn new(led: Led, button: Button, delay: Delay, random: Random, timer: Timer) -> Self {
        let mut g = Self {
            led,
            button,
            delay,
            random,
            timer,
            level: 0,
            state: Vec::new(),
        };

        g.new_game();

        g
    }

    pub fn play(&mut self) -> GameResult {
        let mut guesses: Vec<Signal, 64> = Vec::new();
        let debug_string: String<64> = self.state.iter().map(char::from).collect();
        for signal in self.state.iter() {
            Self::blink_signal(&mut self.led, &mut self.delay, signal)
        }

        debug!("Current combination: {}", debug_string);

        while guesses.len() < self.state.len() {
            let signal = self.read_signal();
            let Some(signal) = signal else {
                continue;
            };
            guesses.push(signal).unwrap();
        }
        let debug_string: String<64> = self.state.iter().map(char::from).collect();
        debug!("Guessed combination: {}", debug_string);

        (guesses == self.state).into()
    }

    pub fn advance(&mut self) -> u8 {
        self.level += 1;
        let new_signal = self.generate_next_signal();
        self.state.push(new_signal).unwrap();

        self.level
    }

    pub fn new_game(&mut self) {
        self.level = 1;
        let new_signal = self.generate_next_signal();
        self.state.clear();
        self.state.push(new_signal).unwrap();
    }

    fn generate_next_signal(&mut self) -> Signal {
        Signal::from(self.random.next_u32())
    }

    // To avoid borrowing self as mutable and immutable reference during iterating over signal array
    fn blink_signal(led: &mut Led, delay: &mut Delay, signal: &Signal) {
        led.set_high().unwrap();
        match signal {
            Signal::Long => delay.delay_ms(2000),
            Signal::Short => delay.delay_ms(500),
        }
        led.set_low().unwrap();
        delay.delay_ms(250);
    }

    fn read_signal(&mut self) -> Option<Signal> {
        while self.button.is_low().unwrap() {}
        self.timer.start(1.hours()).unwrap();
        while self.button.is_high().unwrap() {}
        let duration = self.timer.now();
        info!("Time: {}", duration);
        self.timer.cancel().unwrap();
        Signal::from_duration(duration.duration_since_epoch())
    }
}

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
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
        if game.play() == GameResult::Correct {
            let level = game.advance();
            info!("Correct! Next level: {}", level);
        } else {
            info!("Incorrect! Starting new game.");
            game.new_game();
        }
    }
}
