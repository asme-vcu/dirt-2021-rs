//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]

//use cortex_m::prelude::_embedded_hal_serial_Read;
use rp_pico as bsp;

use bsp::entry;
use defmt::*;
use defmt_rtt as _;
//use embedded_hal::digital::v2::OutputPin;
use panic_probe as _;

use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock},
    pac,
    sio::Sio,
    uart,
    watchdog::Watchdog,
};

#[entry]
fn main() -> ! {
    info!("Program start");
    let mut pac = pac::Peripherals::take().unwrap();
    //let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    // External high-speed crystal on the pico board is 12Mhz
    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    //let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    //let mut led_pin = pins.led.into_push_pull_output();

    let uart_pins = (pins.gpio0.into_mode(), pins.gpio1.into_mode());

    let ibus = uart::UartPeripheral::new(pac.UART0, uart_pins, &mut pac.RESETS)
        .enable(uart::UartConfig::default(), clocks.peripheral_clock.freq())
        .unwrap();

    loop {
        let mut buff = [0; 0x20];

        if ibus.read_full_blocking(&mut buff).is_ok() {
            info!("Read bytes {=[u8; 32]:02X}", buff);
        }
    }
}
