#![no_std]
#![no_main]
#![feature(exhaustive_patterns)]
#![feature(stmt_expr_attributes)]

//use panic_halt as _; // breakpoint on `rust_begin_unwind` to catch panics
use panic_semihosting as _;

use cortex_m_rt::entry;
use display_interface_spi::SPIInterface;
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::Rgb565,
    prelude::*,
    text::{Baseline, Text},
};
use st7789::{self, Orientation};
use stm32f3xx_hal::{
    block, pac,
    prelude::*,
    spi,
    time::{duration::Seconds, rate::Megahertz},
    timer::Timer,
};

#[entry]
fn main() -> ! {
    let core_peripherals = cortex_m::peripheral::Peripherals::take().unwrap();
    let peripherals = pac::Peripherals::take().unwrap();
    let mut reset_and_clock_control = peripherals.RCC.constrain();
    let mut flash = peripherals.FLASH.constrain();
    let clocks = reset_and_clock_control
        .cfgr
        .sysclk(Megahertz(64))
        .pclk2(Megahertz(64))
        .freeze(&mut flash.acr);
    let mut timer = Timer::new(peripherals.TIM1, clocks, &mut reset_and_clock_control.apb2);

    // For determining which bus (ahb) is needed, section 3.2.2 in
    // https://www.st.com/resource/en/reference_manual/dm00043574-stm32f303xb-c-d-e-stm32f303x6-8-stm32f328x8-stm32f358xc-stm32f398xe-advanced-arm-based-mcus-stmicroelectronics.pdf
    // documents which peripherals are reachable over which buses.
    let mut gpioa = peripherals.GPIOA.split(&mut reset_and_clock_control.ahb);

    let sclk = gpioa
        .pa5
        .into_af_push_pull::<5>(&mut gpioa.moder, &mut gpioa.otyper, &mut gpioa.afrl);
    let miso = gpioa
        .pa6
        .into_af_push_pull::<5>(&mut gpioa.moder, &mut gpioa.otyper, &mut gpioa.afrl);
    let mosi = gpioa
        .pa7
        .into_af_push_pull::<5>(&mut gpioa.moder, &mut gpioa.otyper, &mut gpioa.afrl);

    let spi_config = spi::config::Config::default().frequency(Megahertz(20));
    let spi = spi::Spi::new(
        peripherals.SPI1,
        (sclk, miso, mosi),
        spi_config,
        clocks,
        &mut reset_and_clock_control.apb2,
    );

    let backlight = gpioa.pa0.into_push_pull_output(&mut gpioa.moder, &mut gpioa.otyper);
    let data = gpioa.pa2.into_push_pull_output(&mut gpioa.moder, &mut gpioa.otyper);
    let reset = gpioa.pa3.into_push_pull_output(&mut gpioa.moder, &mut gpioa.otyper);
    let chip_select = gpioa.pa4.into_push_pull_output(&mut gpioa.moder, &mut gpioa.otyper);

    let spi_interface = SPIInterface::new(spi, data, chip_select);
    let mut display = st7789::ST7789::new(spi_interface, Some(reset), Some(backlight), 240, 240);

    let mut delay = cortex_m::delay::Delay::new(core_peripherals.SYST, clocks.hclk().0);
    display.init(&mut delay).unwrap();

    let character_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(Rgb565::RED)
        .build();

    display.clear(Rgb565::BLACK).unwrap();
    Text::with_baseline("hello world", Point::new(0, 0), character_style, Baseline::Top)
        .draw(&mut display)
        .unwrap();
    timer.start(Seconds(2));
    block!(timer.wait()).unwrap();

    let colours = [
        Rgb565::RED,
        Rgb565::GREEN,
        Rgb565::BLUE,
        Rgb565::YELLOW,
        Rgb565::CSS_PURPLE,
    ];
    let orientations = [
        Orientation::Portrait,
        Orientation::Landscape,
        Orientation::PortraitSwapped,
        Orientation::LandscapeSwapped,
    ];
    let mut orientation_index = 0;
    loop {
        for colour in colours {
            display.set_orientation(orientations[orientation_index]).unwrap();
            orientation_index = (orientation_index + 1) % orientations.len();
            display.clear(colour).unwrap();
        }
    }
}
