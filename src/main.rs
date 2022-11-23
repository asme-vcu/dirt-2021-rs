#![no_std]
#![no_main]

#[rtic::app(device = rp_pico::hal::pac, peripherals = true, dispatchers = [I2C1_IRQ])]
mod app {
    use defmt::*;
    use embedded_hal::digital::v2::{OutputPin, ToggleableOutputPin};
    use joe_dirt_pico::fs_ia6b_driver::Driver as Reciever;
    use rp2040_monotonic::{ExtU64, Rp2040Monotonic};
    use rp_pico::{
        hal::{self, clocks::init_clocks_and_plls, gpio, uart, watchdog::Watchdog, Clock, Sio},
        XOSC_CRYSTAL_FREQ,
    };

    type UartPin<P> = gpio::Pin<P, gpio::Function<gpio::Uart>>;
    type Uart<P1, P2> =
        uart::UartPeripheral<uart::Enabled, hal::pac::UART0, (UartPin<P1>, UartPin<P2>)>;

    #[monotonic(binds = TIMER_IRQ_0, default = true)]
    type MyMono = Rp2040Monotonic;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        led: gpio::Pin<gpio::bank0::Gpio25, gpio::PushPullOutput>,
        reciever: Reciever<Uart<gpio::bank0::Gpio0, gpio::bank0::Gpio1>>,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        // reset spinlocks (normally called by #[hal::entry])
        unsafe {
            hal::sio::spinlock_reset();
        }

        // init system control registers
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

        // initialize monotonic timer
        let mono = Rp2040Monotonic::new(cx.device.TIMER);

        // init GPIO pins
        let sio = Sio::new(cx.device.SIO);
        let pins = rp_pico::Pins::new(
            cx.device.IO_BANK0,
            cx.device.PADS_BANK0,
            sio.gpio_bank0,
            &mut resets,
        );

        // configure LED pin
        let mut led = pins.led.into_push_pull_output();
        led.set_low().unwrap();

        // start LED task
        toggle_led::spawn_after(1.secs()).unwrap();

        // configure UART interface
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

        // enable UART interrupts
        uart.enable_rx_interrupt();

        // initialize reciever
        let reciever = Reciever::new(uart);

        // finish init
        (Shared {}, Local { led, reciever }, init::Monotonics(mono))
    }

    // idle task puts processor in sleep mode to save energy
    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            cortex_m::asm::wfi();
        }
    }

    // LED task simply toggles an LED to make sure everything's OK
    #[task(
        local = [led],
    )]
    fn toggle_led(cx: toggle_led::Context) {
        cx.local.led.toggle().unwrap();

        // respawn task
        toggle_led::spawn_after(1.secs()).unwrap();
    }

    #[task(
        binds = UART0_IRQ,
        local = [count: usize = 0, reciever]
    )]
    fn uart_irq(cx: uart_irq::Context) {
        match cx.local.reciever.read() {
            Ok(output) => {
                if let Some(dat) = output {
                    // display every 20th packet
                    // this reduces console spam
                    if *cx.local.count % 20 == 0 {
                        info!("Recieved Channel data: {=[?; 14]}", dat);
                    }
                    *cx.local.count += 1;
                }
            }
            Err(e) => {
                error!("{}", e);
            }
        }
    }
}
