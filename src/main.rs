#![no_std]
#![no_main]

use defmt_rtt as _; // global logger
use panic_probe as _; // panic handler

#[rtic::app(device = rp_pico::hal::pac, peripherals = true)]
mod app {
    use defmt::*;
    use embedded_hal::digital::v2::OutputPin;
    use fugit::SecsDurationU32;
    use rp_pico::{
        hal::{
            self, clocks::init_clocks_and_plls, gpio, timer::Alarm, watchdog::Watchdog, Clock, Sio,
        },
        XOSC_CRYSTAL_FREQ,
    };

    type UartPin<P> = gpio::Pin<P, gpio::Function<gpio::Uart>>;
    type Uart<P1, P2> =
        hal::uart::UartPeripheral<hal::uart::Enabled, hal::pac::UART0, (UartPin<P1>, UartPin<P2>)>;

    const SCAN_TIME_US: SecsDurationU32 = SecsDurationU32::secs(1);

    #[shared]
    struct Shared {
        timer: hal::Timer,
        alarm: hal::timer::Alarm0,
        led: gpio::Pin<gpio::bank0::Gpio25, gpio::PushPullOutput>,
        uart: Uart<gpio::bank0::Gpio0, gpio::bank0::Gpio1>,
    }

    #[local]
    struct Local {}

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        unsafe {
            hal::sio::spinlock_reset();
        }

        let mut resets = cx.device.RESETS;
        let mut watchdog = Watchdog::new(cx.device.WATCHDOG);
        let clocks = init_clocks_and_plls(
            XOSC_CRYSTAL_FREQ,
            cx.device.XOSC,
            cx.device.CLOCKS,
            cx.device.PLL_SYS,
            cx.device.PLL_USB,
            &mut resets,
            &mut watchdog,
        )
        .ok()
        .unwrap();

        let sio = Sio::new(cx.device.SIO);
        let pins = rp_pico::Pins::new(
            cx.device.IO_BANK0,
            cx.device.PADS_BANK0,
            sio.gpio_bank0,
            &mut resets,
        );
        let mut led = pins.led.into_push_pull_output();
        led.set_low().unwrap();

        let mut timer = hal::Timer::new(cx.device.TIMER, &mut resets);
        let mut alarm = timer.alarm_0().unwrap();
        let _ = alarm.schedule(SCAN_TIME_US);
        alarm.enable_interrupt();

        let mut uart = hal::uart::UartPeripheral::new(
            cx.device.UART0,
            (pins.gpio0.into_mode(), pins.gpio1.into_mode()),
            &mut resets,
        )
        .enable(
            hal::uart::common_configs::_115200_8_N_1,
            clocks.peripheral_clock.freq(),
        )
        .unwrap();

        uart.enable_rx_interrupt();

        (
            Shared {
                timer,
                alarm,
                led,
                uart,
            },
            Local {},
            init::Monotonics(),
        )
    }

    #[task(
        binds = TIMER_IRQ_0,
        priority = 1,
        shared = [timer, alarm, led],
        local = [tog: bool = true],
    )]
    fn timer_irq(mut cx: timer_irq::Context) {
        if *cx.local.tog {
            cx.shared.led.lock(|l| l.set_high().unwrap());
        } else {
            cx.shared.led.lock(|l| l.set_low().unwrap());
        }

        *cx.local.tog = !*cx.local.tog;

        cx.shared.alarm.lock(|a| {
            a.clear_interrupt();
            let _ = a.schedule(SCAN_TIME_US);
        })
    }

    #[task(
        binds = UART0_IRQ,
        priority = 1,
        shared = [uart],
        local = []
    )]
    fn uart_irq(mut cx: uart_irq::Context) {
        let mut buff = [0; 0x20];

        cx.shared.uart.lock(|u| {
            if u.read_full_blocking(&mut buff).is_ok() {
                info!("Read bytes {=[u8; 32]:02X}", buff);
            } else {
                error!("Couldn't read bytes...");
            }
        })
    }
}
