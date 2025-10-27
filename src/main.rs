#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]

//Add this to Cargo.toml if you want to use SysTick as monotonic timer
//[dependencies.rtic-monotonics]
//version = "2.0"
//features = ["cortex-m-systick"]

use defmt_rtt as _;
use panic_probe as _;
use rtic_time::Monotonic;
use stm32f4xx_hal::{
    gpio::{Output, PC13},
    pac,
    prelude::*,
    spi::{Mode, Phase, Polarity, Spi},
};

use display_interface_spi::*;
use ili9341::*;

use embedded_hal_bus::spi::*;

mod images;
mod image_names2;

//TODO
//put RLE into a build.rs to create the image files

//type Mono = stm32f4xx_hal::timer::MonoTimerUs<pac::TIM3>;

// To use SysTick as monotonic timer, uncomment the lines below
// *and* remove the Mono type alias above
use rtic_monotonics::systick::prelude::*;
systick_monotonic!(Mono, 1000);


struct RleImage {
    data: &'static [u16],
    internal_index: usize,
    internal_count: usize,
}

impl RleImage {
    fn new(data: &'static [u16]) -> Self {
        RleImage {
            data: data,
            internal_count: 0,
            internal_index: 0,
        }
    }
}

impl Iterator for &mut RleImage {
    type Item = u16; // The type of elements this iterator will produce

    fn next(&mut self) -> Option<Self::Item> {
        //if all chunks aren't consumed
        if self.internal_index < self.data.len() - 1 {
            //if internal count is less than run_length - 2, transmit run_data and increment the counter
            if self.internal_count < (self.data[self.internal_index + 1] - 1) as usize {
                let value = self.data[self.internal_index];
                self.internal_count += 1;
                Some(value)
            } else {
                // internal count = run_length -1, transmit one last time then go to next run
                let value = self.data[self.internal_index];
                self.internal_count = 0;
                self.internal_index += 2;
                Some(value)
            }
        } else {
            //ran all runs, done.
            self.internal_count = 0;
            self.internal_index = 0;
            None
        }
    }
}

#[derive(Debug, defmt::Format)]
    enum AnimationState {
        ActiveAnimation,
        IdleAnimation,
    }

use rtic::app;

#[app(device = pac, dispatchers = [USART1], peripherals = true)]
mod app {

    use stm32f4xx_hal::rcc::Config;

    use super::*;
    

    #[shared]
    struct Shared {
        sensor_flag: bool,
    }

    #[local]
    struct Local {
        led: PC13<Output>,
        display: Ili9341<
            SPIInterface<
                ExclusiveDevice<
                    Spi<stm32f4::Periph<pac::spi1::RegisterBlock, 1073819648>>,
                    stm32f4xx_hal::gpio::Pin<'B', 10, Output>,
                    NoDelay,
                >,
                stm32f4xx_hal::gpio::Pin<'A', 8, Output>,
            >,
            stm32f4xx_hal::gpio::Pin<'A', 9, Output>,
        >,
        pir_sensor: stm32f4xx_hal::gpio::Pin<'B', 0>,
    }

    #[init]
    fn init(mut ctx: init::Context) -> (Shared, Local) {

        // Uncomment if use SysTick as monotonic timer
        Mono::start(ctx.core.SYST, 48_000_000);

        let mut rcc = ctx.device.RCC.freeze(Config::hsi().sysclk(48.MHz()));
        let gpioa = ctx.device.GPIOA.split(&mut rcc);
        let gpiob = ctx.device.GPIOB.split(&mut rcc);
        let gpioc = ctx.device.GPIOC.split(&mut rcc);

        let mut pir_sensor = gpiob.pb0.into_input();
        let mut syscfg = ctx.device.SYSCFG.constrain(&mut rcc);
        pir_sensor.make_interrupt_source(&mut syscfg);

        pir_sensor.trigger_on_edge(
            &mut ctx.device.EXTI,
            stm32f4xx_hal::gpio::Edge::RisingFalling,
        );

        pir_sensor.enable_interrupt(&mut ctx.device.EXTI);

        let led = gpioc.pc13.into_push_pull_output();

        let sclk = gpiob.pb3.into_alternate();

        let mosi = gpiob.pb5.into_alternate();
        let cs = gpiob.pb10.into_push_pull_output();
        let dc = gpioa.pa8.into_push_pull_output();
        let reset_gpio = gpioa.pa9.into_push_pull_output();
        gpioc
            .pc7
            .into_push_pull_output_in_state(stm32f4xx_hal::gpio::PinState::High);

        let spi: Spi<stm32f4::Periph<stm32f4xx_hal::pac::spi1::RegisterBlock, 1073819648>> =
            ctx.device.SPI1.spi(
                (Some(sclk), stm32f4xx_hal::pac::SPI1::NoMiso, Some(mosi)),
                Mode {
                    polarity: Polarity::IdleLow,
                    phase: Phase::CaptureOnFirstTransition,
                },
                32.MHz(),
                &mut rcc,
            );

        let spi_device = ExclusiveDevice::new_no_delay(spi, cs).unwrap();

        let iface = SPIInterface::new(spi_device, dc);
        let display = Ili9341::new(
            iface,
            reset_gpio,
            &mut Mono,
            Orientation::PortraitFlipped,
            ili9341::DisplaySize240x320,
        )
        .unwrap();
        defmt::info!("Start");

        let sensor_flag = false;

        tick::spawn().ok();
        (
            Shared { sensor_flag },
            Local {
                led,
                display,
                pir_sensor,
            },
        )
    }

    #[task(local = [led, display, count: u32 = 0], shared = [sensor_flag])]
    async fn tick(mut ctx: tick::Context) {
        let mut flip = false;
        let mut state = AnimationState::IdleAnimation;

        let mut ACTIVE_1 = RleImage::new(images::ACTIVE_1);
        let mut ACTIVE_2 = RleImage::new(images::ACTIVE_2);
        let mut IDLE_1 = RleImage::new(images::IDLE_1);
        let mut IDLE_2 = RleImage::new(images::IDLE_2);

        loop {
            ctx.local.led.toggle();
            //*ctx.local.count += 1;
            defmt::info!("Tick {} {} {}", *ctx.local.count, state, flip);

            let image = match (&state, flip) {
                (AnimationState::ActiveAnimation, true) => &mut ACTIVE_1,
                (AnimationState::ActiveAnimation, false) => &mut ACTIVE_2,
                (AnimationState::IdleAnimation, true) => &mut IDLE_1,
                (AnimationState::IdleAnimation, false) => &mut IDLE_2,
            };

            ctx.local.display.write_iter(image).unwrap();

            ctx.shared.sensor_flag.lock(|flag| {
                state = match *flag {
                    true => AnimationState::ActiveAnimation,
                    false => AnimationState::IdleAnimation,
                }
            });
            flip = !flip;

            Mono::delay(200.millis().into()).await;
        }
    }

    #[task(binds = EXTI0, local = [pir_sensor], shared = [sensor_flag])]
    fn gpio_interrupt_handler(mut ctx: gpio_interrupt_handler::Context) {
        defmt::info!("Interrupt called! {}", ctx.local.pir_sensor.is_high());
        ctx.shared
            .sensor_flag
            .lock(|flag| *flag = ctx.local.pir_sensor.is_high());
        ctx.local.pir_sensor.clear_interrupt_pending_bit();
    }
}
