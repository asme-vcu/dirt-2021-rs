#![no_std]
#![no_main]

pub use defmt_rtt as _; // global logger
pub use panic_probe as _; // panic handler

pub mod flysky_driver;

#[rtic::app(device = rp_pico::hal::pac, peripherals = true)]
mod app {
    use crate::flysky_driver::Driver as Transciever;
    use crate::flysky_driver::Error as TranscieverError;
    use defmt::*;
    use embedded_hal::digital::v2::OutputPin;
    use fugit::SecsDurationU32;
    use rp_pico::{
        hal::{
            self, clocks::init_clocks_and_plls, gpio, timer::Alarm, uart, watchdog::Watchdog,
            Clock, Sio,
        },
        XOSC_CRYSTAL_FREQ,
    };

    type UartPin<P> = gpio::Pin<P, gpio::Function<gpio::Uart>>;
    type Uart<P1, P2> =
        uart::UartPeripheral<uart::Enabled, hal::pac::UART0, (UartPin<P1>, UartPin<P2>)>;

    const SCAN_TIME_US: SecsDurationU32 = SecsDurationU32::secs(1);

    #[shared]
    struct Shared {
        timer: hal::Timer,
        alarm: hal::timer::Alarm0,
        led: gpio::Pin<gpio::bank0::Gpio25, gpio::PushPullOutput>,
        transciever: Transciever<Uart<gpio::bank0::Gpio0, gpio::bank0::Gpio1>>,
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

        let mut uart = uart::UartPeripheral::new(
            cx.device.UART0,
            (pins.gpio0.into_mode(), pins.gpio1.into_mode()),
            &mut resets,
        )
        .enable(
            uart::common_configs::_115200_8_N_1,
            clocks.peripheral_clock.freq(),
        )
        .unwrap();

        uart.enable_rx_interrupt();

        let transciever = Transciever::new(uart);

        (
            Shared {
                timer,
                alarm,
                led,
                transciever,
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
        shared = [transciever],
        local = [count: usize = 0]
    )]
    fn uart_irq(mut cx: uart_irq::Context) {
        cx.shared.transciever.lock(|t| match t.read() {
            Ok(output) => {
                if let Some(dat) = output {
                    //if *cx.local.count % 6 == 0 {
                    info!("Recieved Channel data: {=[?; 14]}", dat);
                    //}
                    *cx.local.count += 1;
                }
            }
            Err(e) => match e {
                TranscieverError::SerialError(_) => {
                    error!("error while communicating with the bus");
                }
                TranscieverError::WouldBlock => {
                    error!("operation would cause blocking");
                }
                TranscieverError::InvalidLength(got, expected) => {
                    warn!(
                        "invalid packet length: got {=u8}, expected {=u8}",
                        got, expected
                    );
                }
                TranscieverError::InvalidCommand(got, expected) => {
                    warn!(
                        "invalid packet command: got {=u8}, expected {=u8}",
                        got, expected
                    );
                }
                TranscieverError::InvalidChecksumL(got, expected) => {
                    warn!(
                        "invalid checksum lower byte: got {=u8}, expected {=u8}",
                        got, expected
                    );
                }
                TranscieverError::InvalidChecksumH(got, expected) => {
                    warn!(
                        "invalid checksum upper byte: got {=u8}, expected {=u8}",
                        got, expected
                    );
                }
            },
        });
    }
}
