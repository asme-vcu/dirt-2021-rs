# 1. dirt-2021-rs

- [1. dirt-2021-rs](#1-dirt-2021-rs)
  - [1.1. Why the Pico?](#11-why-the-pico)
  - [1.2. Arduino framework shortcomings](#12-arduino-framework-shortcomings)
  - [1.3. Introducing Rust and RTIC](#13-introducing-rust-and-rtic)
  - [1.4. Dependency Overview](#14-dependency-overview)
  - [1.5. Running](#15-running)
  - [1.6. Wiring Guide (WIP)](#16-wiring-guide-wip)
  - [1.7. Sidenote](#17-sidenote)
  - [1.8. Final Verdict](#18-final-verdict)
    - [1.8.1. Is there a noticeable performance difference with the Rust version?](#181-is-there-a-noticeable-performance-difference-with-the-rust-version)
    - [1.8.2. So if there isn't a performance gain, is Rust still worth it?](#182-so-if-there-isnt-a-performance-gain-is-rust-still-worth-it)
  - [1.9. Side note: Learning curve](#19-side-note-learning-curve)

All the code for Joe Dirt rewritten in Rust...

(This is Nathan's new hobby project)

## 1.1. Why the Pico?

Our original robot ran code on the Arduino Due and was programmed using the Arduino language. While the code worked fine and was easy to understand, the Due ended up being a burden in more ways than one. It uses a 7V input, is not power efficient, and despite having 54 pins, many of them have no supplemental hardware functions. This, combined with a high price, and the shortcomings of the Arduino framework (see next section) led to it being a less-than-idea microcontroller.

Even though the Raspberry Pico and the Due came out around the same time, the Pico is far superior as a microcontroller board. It has a smaller footprint, a faster processor (even dual core!), uses less power, and is far cheaper. On top of this, almost every single GPIO pin supports half a dozen different features including UART, SPI, I2C, and PWM, allowing for far more flexibility despite having less of them. Its superior to the Due in almost every single way, so switching to it was a no-brainer. The only question is, do we want to keep using the arduino framework?

## 1.2. Arduino framework shortcomings

On the surface, the Arduino framework seems fine. Its relatively easy to use, has a ton of tutorials, and is implemented in C/C++, the industry standard language for embedded applications. So what problems did I run into while using it?

 - Virtually no compile-time checking of anything
 - No integrated unit testing
 - High layers of abstraction restrict control and are very opaque
 - Difficult/impossible to implement multi-tasking
 - No message passing - heavy use of global variables
 - Custom drivers are difficult to read & understand
 - Heavy reliance on shared user-made code without verifying source code
 - Unlicensed dependencies

In general, its not production-ready code. While it passes for hobbyists and children, I think engineering students can do much better.

## 1.3. Introducing Rust and RTIC

Rust is a new programming language backed by several of the largest tech companies. While its young, it has many promising features and advantages over C/C++:

 - Compile-time checking of as much as possible
 - Integrated unit testing
 - All libraries must be have open source licenses to be published
 - Heavy emphasis on code documentation & correctness
 - Source code visible alongside documentation
 - Explicit error handling
 - Massive community efforts to make a robust embedded ecosystem

Of course, there are more features that I'm not going into, but these are some of the most relevant ones. Here are some of the features of the evolving ecosystem:

 - Platform agnostic drivers using trait-based APIs
 - Multi-layer hardware support
    - Peripheral Access (PAC): Device register mappings
    - Hardware Abstraction Layer (HAL): Safe & lightweight abstractions over hardware
    - Board Support (BSP): GPIO pin mappings, integrated hardware, etc
 - Drop-in replaceable panic handlers allow for different hard-fault behaviors
 - Drop-in replaceable logging backends

But the crowning jewel is the RTIC framework. RTIC stands for Real-Time Interrupt-driven Concurrency. It has several advanced features that put Arduino to shame:

 - Easy multi-threading via "tasks"
 - Easy to use software task scheduling
 - Task pre-emption
 - Easy to use hardware interrupts
 - Message passing between tasks
 - Shared and local resources for tasks
 - Thread-safety out the box

Despite having these advanced features, I find its surprisingly easy to read and digest compared to C. A lot of the syntax and boiler-plate is simplified by Rust's very powerful macro system. The trait system is both easier to understand than abstract classes and far more powerful. Driver development is very straight-forward and once a driver is created it can be used across hundreds of different microcontrollers thanks to the platform-agnostic API.

## 1.4. Dependency Overview

 - `cortex-m` allows in-place assembly and more
 - `cortex-m-rtic` is an implementation of the RTIC (Real-Time Interrupt-driven Concurrency) framework for cortex-m devices. It gives us a safe asbtraction over multithreaded applications by using software tasks & interrupt handlers. By using interrupt priorities, it also allows for pre-emptive multitasking.
 - `defmt` (defferred formatting) is a logging framework for embedded devices
 - `defmt-rtt` allows transferring `defmt` messages over the RTT protocol
 - `embedded-hal` creates traits for platform-agnostic driver implementation
 - `flip-link` swaps the stack and data sectors to have stack overflow detection during runtime
 - `fugit` is an embedded time library, allowing for type-enforced unit conversion with no runtime overhead
 - `mpu6050` is a driver for our IMU
 - `nb` provides traits for blocking/non-blocking APIs
 - `panic-probe` prints a backtrace inside the probe debugger on panics
 - `probe-run` allows for running with debugging/semihosting using cargo
 - `rp-pico` is a board-access crate, mapping processor outputs to physical board pinouts and more
 - `rp2040-monotonic` implements a monotonic timer using hardware on the RP2040 board for task scheduling in the RTIC framework

## 1.5. Running

Follow the steps [here](https://github.com/rp-rs/rp2040-project-template). Note that you'll need a debugger to run the code interactively, the easiest method is to use a second pico as a debugger.

## 1.6. Wiring Guide (WIP)

Host Pi:

 - FS-iA6B i-bus: GPIO1
 - MPU-6050 SDA: GPIO2
 - MPU-6050 SCK: GPIO3
 - MPU-6050 INT: GPIO4
 - Front left PWM: GPIO6
 - Back left PWM: GPIO7
 - Front right PWM: GPIO8
 - Back right PWM: GPIO9

Debugger:

 - SWCLK: GPIO2
 - SWDIO: GPIO3

## 1.7. Sidenote

There were two main goals to this Rust port. The first was to explore the viability of Rust and the Pico for future designs. The second was to improve upon and correct issues with Joe Dirt's design. However, upon completion, I learned a rather dissapointing reality. The major issues in Joe Dirt's design largely had nothing to do with the microcontroller or my original code. They were each caused by external factors:

- The motors operate poorly at low speeds. This is due to the way BLDC drivers have traditionally worked. Modern commercial ESC's don't properly modulate the signal at lower speeds, and are only stable at high speeds.
- Each ESC and motor is slightly different, causing some wheels to jitter while others remain stationary, amongst other issues. None of the wheels run at exactly the same speed even when they should.
- The motors have a high input latency. This is also caused by the ESCs: they operate on a PWM signal with a 50Hz (20ms) refresh. This is extremely slow even compared to the refresh rate of the reciever (~7ms).
- The wiring inside the chassis is awful. This is a combination of several factors, including low-quality wire, poor cable management planning, etc. However, one of the main culprits is the fact that we have 4 separate ESC's spread throughout the chassis, connected to a large centralized bus bar. In such a tight build, it would've made far more sense logistically to combine all four ESC's and the bus bar into one single motor-driver board. This could've easily fit within the space in the chassis, had we the time and expertise to design such a board.

How do we actually solve this issue? By either spending a lot of money on ESC's and meticulously planning the electronics layout within the chassis, or by making an in-house motor driver board. Thanks to recent advancements in FOC technology, this is a more viable option now than ever, and I strongly believe we need to invest in this direction to create more competitive cutting-edge designs.

## 1.8. Final Verdict

### 1.8.1. Is there a noticeable performance difference with the Rust version?

Yes and no.

Performance-wise, the Joe Dirt is severely bottlenecked by the ESCs. However, we do have much more responsive code. While debugging, I found that there was no lag whatsoever while printing over a hundred lines to the console per second, which is a very different story from the Arduino. The entire robot now runs all the different tasks in real-time, rather than waiting on each to complete in a predefined order. If the ESCs didn't rely on 1980s technology, and instead used modern FOC with an efficient communications protocol, then we would be able to reap far more practical gains.

### 1.8.2. So if there isn't a performance gain, is Rust still worth it?

In my opinion, yes very much so. I've already discussed many of the benefits previously, but just to reiterate:

 - It's easier to handle many different tasks at once
 - There's pre-emptive multitasking
 - Gives far more control & flexibility
 - More transparent to actual hardware
 - Compatible with pretty much all hardware, espcially Cortex-M based
 - The control logic is easier to read & understand (in my opinion)

So, if performance is a concern and there are ECE students on the power systems team, especially ones that have taken 364, I would recommend rust to be used.

## 1.9. Side note: Learning curve

I started learning Rust a few months after the original Joe Dirt code was written. Over the summer, I learned all of the ins-and-outs of the Rust language, and at the time of writing, I'm currently taking EGRE-364 as a junior. This is the first project I've written in embedded Rust, and I did all the learning in my free time this semester. How long or difficult this process is for other people remains to be seen, but I think with about 5-6 weeks of dedication it can be done.

Because there are so many dependencies to this project, and you need some level of understanding for each of them, this amount of time is highly variable. I hope that this project can serve as a good resource in the future, and that I'll be able to train the next generation of students with the knowledge I've gained.

Here are some of the most useful resources I've used:

 - (The Rust Book)[https://doc.rust-lang.org/stable/book/]. This is the first line of knowledge to learn Rust. Because this is a `no_std` application, not all of this information is relevant. However, I'd recommend reading it front-to-back anyways as its one of the most detailed resources and is fundamental to understanding the more complex information.
 - (The RP2040 HAL)[https://github.com/rp-rs/rp-hal#gettting_started]. This project makes all of the Raspberry Pico compatibility happen. I'd recommend reading through it and the quickstart project. It also links to several more important resources, including the `embedded_hal` api docs.
 - (RTIC user documetation)[https://rtic.rs/1/book/en/]. This explains how RTIC works and everything you need to know while fleshing out your own embedded projects.
 - (Raspberry Pico Examples)[https://github.com/rp-rs/rp-hal/tree/main/boards/rp-pico/examples]. These include several examples for doing common tasks. Be sure to pay attention to the RTIC ones!
 - (The API docs)[https://docs.rs/]. This is the programming documentation on every single 3rd party library published on `crates.io`. Some of it can vary in readability, especially the generics- and macro- heavy crates. Be sure to check for user-level documentation before the API-level documentation. **Note:** every function also has the source code available. If you need or want to see how something is implemented, by all means read their code (crates often use complex internal macros though).
