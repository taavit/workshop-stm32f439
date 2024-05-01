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
use heapless::Vec;
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
#[derive(Debug, PartialEq)]
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

impl From<bool> for Signal {
    fn from(value: bool) -> Self {
        match value {
            false => Signal::Short,
            true => Signal::Long,
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

trait Parity {
    fn is_even(&self) -> bool;
    fn is_odd(&self) -> bool;
}
impl Parity for u32 {
    fn is_even(&self) -> bool {
        self % 2 == 0
    }

    fn is_odd(&self) -> bool {
        self % 2 == 1
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
    let mut delay = cp.SYST.delay(&clocks);
    let mut timer = dp.TIM2.counter_ms(&clocks);

    let adc: Adc<ADC1> = Adc::adc1(dp.ADC1, false, AdcConfig::default());
    let analog_pin = gpiob.pb0.into_analog();
    let mut rng = RandomFromNoise::new(adc, analog_pin);

    let mut led = gpiob.pb7.into_push_pull_output();
    let mut button = gpioc.pc13.into_pull_down_input();

    let mut level = 1;
    let mut chain = Vec::<Signal, 64>::new();

    loop {
        let mut guesses = 0;
        info!("Level {}", level);
        let mut answer = Vec::<Signal, 64>::new();
        debug!("Generated chain: {}", chain);
        chain
            .iter()
            .for_each(|signal| blink_signal(&mut led, &mut delay, signal));
        while guesses < chain.len() {
            let signal = read_signal(&mut button, &mut timer);
            let Some(signal) = signal else {
                continue;
            };
            answer.push(signal).unwrap();
            guesses += 1;
        }
        debug!("Answered chain: {}", answer);
        if answer == chain {
            info!("Level completed!");
            chain.push(generate_next_signal(&mut rng)).unwrap();
            level += 1;
        } else {
            info!("Level failed! Expected chain: {}", chain);
            info!("New game!");
            chain.clear();
            chain.push(generate_next_signal(&mut rng)).unwrap();
            level = 1;
        }
    }
}

fn generate_next_signal<R: RngCore>(rnd: &mut R) -> Signal {
    Signal::from(rnd.next_u32().is_odd())
}

fn blink_signal<Pin: OutputPin, Delayer: DelayNs>(
    p: &mut Pin,
    delay: &mut Delayer,
    signal: &Signal,
) {
    p.set_high().unwrap();
    match signal {
        Signal::Long => delay.delay_ms(2000),
        Signal::Short => delay.delay_ms(500),
    }
    p.set_low().unwrap();
    delay.delay_ms(250);
}

fn read_signal<const TIMER_HZ: u32, Pin: InputPin, Timer: fugit_timer::Timer<TIMER_HZ>>(
    p: &mut Pin,
    t: &mut Timer,
) -> Option<Signal> {
    while p.is_low().unwrap() {}
    t.start(1.hours()).unwrap();
    while p.is_high().unwrap() {}
    let duration = t.now();
    info!("Time: {}", duration);
    t.cancel().unwrap();
    Signal::from_duration(duration.duration_since_epoch())
}
