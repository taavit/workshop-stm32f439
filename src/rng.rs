use embedded_hal_02::adc::Channel;
use rand_core::RngCore;
use stm32f4xx_hal::{
    adc::{config::SampleTime, Adc},
    pac::ADC1,
};

pub struct RandomFromNoise<PIN: Channel<ADC1>> {
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
