#![no_std]

use defmt::{debug, info, Format};
use embedded_hal::{
    delay::DelayNs,
    digital::{InputPin, OutputPin},
};
use fugit::{Duration, ExtU32};
use game::{Game, GameResult};
use heapless::{String, Vec};
use rand_core::RngCore;

pub mod game;
pub mod rng;

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

pub struct MemoryGame<
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
    > Game for MemoryGame<TIMER_HZ, Random, Delay, Led, Button, Timer>
{
    fn play(&mut self) -> GameResult {
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

    fn advance(&mut self) -> u8 {
        self.level += 1;
        let new_signal = self.generate_next_signal();
        self.state.push(new_signal).unwrap();

        self.level
    }

    fn new_game(&mut self) {
        self.level = 1;
        let new_signal = self.generate_next_signal();
        self.state.clear();
        self.state.push(new_signal).unwrap();
    }
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
        self.led.set_low().unwrap();
        while self.button.is_low().unwrap() {}
        self.timer.start(1.hours()).unwrap();
        let mut high = false;
        while self.button.is_high().unwrap() {
            if !high
                && self.timer.now().duration_since_epoch() > Duration::<u32, 1, TIMER_HZ>::secs(2)
            {
                self.led.set_high().unwrap();
                high = true;
            }
        }
        let duration = self.timer.now();
        info!("Time: {}", duration);
        self.timer.cancel().unwrap();
        Signal::from_duration(duration.duration_since_epoch())
    }
}
