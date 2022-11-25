#![no_std]
#![no_main]

// Main RTIC app
// Note: dispatchers allow for pre-emptive multitasking, these interrupts cannot be
// reused. In the next update of RP2040_hal, we will be able to use all of the software
// interrupts rather than ones we might actually want.
#[rtic::app(device = rp_pico::hal::pac, peripherals = true, dispatchers = [TIMER_IRQ_1])]
mod app {
    use cortex_m::delay::Delay;
    use defmt::*;
    use embedded_hal::{
        digital::v2::{OutputPin, ToggleableOutputPin},
        PwmPin,
    };
    use fugit::RateExtU32;
    use joe_dirt_pico::fs_ia6b_driver::Driver as Reciever;
    use mpu6050::{Mpu6050, PI_180};
    use rp2040_monotonic::{ExtU64, Rp2040Monotonic};
    use rp_pico::{
        hal::{
            self, clocks::init_clocks_and_plls, gpio, i2c, pwm, uart, watchdog::Watchdog, Clock,
            Sio,
        },
        Gp0Uart0Tx, Gp1Uart0Rx, Gp2I2C1Sda, Gp3I2C1Scl, XOSC_CRYSTAL_FREQ,
    };

    // monotonic timer allows for task scheduling
    // soon this will be implemented inside the `rp2040_hal` itself
    #[monotonic(binds = TIMER_IRQ_0, default = true)]
    type MyMono = Rp2040Monotonic;

    // resources shared between multiple tasks
    #[shared]
    struct Shared {
        fl: pwm::Channel<pwm::Pwm3, pwm::FreeRunning, pwm::A>,
        fr: pwm::Channel<pwm::Pwm4, pwm::FreeRunning, pwm::A>,
        bl: pwm::Channel<pwm::Pwm3, pwm::FreeRunning, pwm::B>,
        br: pwm::Channel<pwm::Pwm4, pwm::FreeRunning, pwm::B>,
    }

    // resources given to individual tasks
    #[local]
    struct Local {
        led: gpio::Pin<gpio::bank0::Gpio25, gpio::PushPullOutput>,
        reciever: Reciever<
            uart::UartPeripheral<uart::Enabled, hal::pac::UART0, (Gp0Uart0Tx, Gp1Uart0Rx)>,
        >,
        imu: Mpu6050<i2c::I2C<hal::pac::I2C1, (Gp2I2C1Sda, Gp3I2C1Scl)>>,
    }

    // initialize device & all resources
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

        // init delay unit
        let mut delay = Delay::new(cx.core.SYST, clocks.system_clock.freq().to_Hz());

        // init monotonic timer
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

        // schedule LED task
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

        // configure I2C interface
        let i2c = i2c::I2C::i2c1(
            cx.device.I2C1,
            pins.gpio2.into_mode(),
            pins.gpio3.into_mode(),
            400.kHz(),
            &mut resets,
            clocks.system_clock.freq(),
        );

        // initialize IMU
        let mut imu = Mpu6050::new(i2c);
        imu.init(&mut delay).unwrap();

        // schedule IMU task
        imu::spawn_after(100.millis()).unwrap();

        // configure PWM
        let slices = hal::pwm::Slices::new(cx.device.PWM, &mut resets);
        let mut pwml = slices.pwm3;
        let mut pwmr = slices.pwm4;
        pwml.default_config(); // default config: 125 MHz frequency
        pwmr.default_config();
        pwml.set_div_int(125); // scale to 1MHz frequency
        pwmr.set_div_int(125);
        pwml.set_top(20_000 - 1); // 20ms period
        pwmr.set_top(20_000 - 1);
        pwml.enable();
        pwmr.enable();

        // PWM pin configurations
        let mut fl = pwml.channel_a;
        let mut fr = pwmr.channel_a;
        let mut bl = pwml.channel_b;
        let mut br = pwmr.channel_b;
        fl.output_to(pins.gpio6);
        bl.output_to(pins.gpio7);
        fr.output_to(pins.gpio8);
        br.output_to(pins.gpio9);
        fl.set_duty(1500); // 1500us = off
        bl.set_duty(1500); // these are bidirectional esc's:
        fr.set_duty(1500); // 1000us = -100%
        br.set_duty(1500); // 2000us =  100%

        // finish init
        (
            Shared { fl, fr, bl, br },
            Local { led, reciever, imu },
            init::Monotonics(mono),
        )
    }

    // idle task puts processor in sleep mode to save energy
    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            cortex_m::asm::wfi(); // wait-for-interrupt
        }
    }

    // toggle an LED to verify the CPU is still running
    #[task(
        local = [led],
    )]
    fn toggle_led(cx: toggle_led::Context) {
        cx.local.led.toggle().unwrap();

        // respawn task
        toggle_led::spawn_after(1.secs()).unwrap();
    }

    // Read the current rotation of the IMU
    // As of now, this is solely for prototyping/informational
    // purposes. The plan is to implement free-fall detection via
    // interrupts & realtime arial orientation corrections by adjusting
    // the acceleration of the wheels
    #[task(local = [imu])]
    fn imu(cx: imu::Context) {
        let _rot = cx.local.imu.get_acc_angles().unwrap() / PI_180;

        // info!("Pitch: {}, Yaw: {}", rot.x, rot.y);

        // respawn task
        imu::spawn_after(100.millis()).unwrap();
    }

    // Respond to UART packets from the reciever
    #[task(
        binds = UART0_IRQ,
        local = [count: usize = 0, reciever],
        shared = [fl, fr, bl, br]
    )]
    fn uart_irq(cx: uart_irq::Context) {
        let uart_irq::SharedResources { fl, fr, bl, br } = cx.shared;

        match cx.local.reciever.read() {
            Ok(output) => {
                if let Some(dat) = output {
                    // calculate arcade drive outputs
                    let left = (dat[1] + dat[0] - 1500).clamp(1000, 2000);
                    let right = (dat[1] + 1500 - dat[0]).clamp(1000, 2000);

                    // write them to ESC's
                    (fl, fr, bl, br).lock(|fl, fr, bl, br| {
                        fl.set_duty(left);
                        fr.set_duty(right);
                        bl.set_duty(left);
                        br.set_duty(right);
                    });

                    // display every 20th packet
                    // this reduces console spam
                    if *cx.local.count % 20 == 0 {
                        info!("Recieved Channel data: {=[?; 14]}", dat);
                        info!("Left: {}, Right: {}", left, right)
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
