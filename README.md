# dirt-2021-rs

All the code for Joe Dirt rewritten in Rust...

(This is Nathan's new hobby project)

# Why Rust?

Our original code was made for the Arduino Due using Arduino's modified C/C++ language. This version was fairly reliable but had noticeable latency and was never fully finished. On top of that, library and driver compatibility was mixed for the Due, and their quality varied widely. There were also various technical limitations that were partially because of Arduino's limited language features: it used a very minimal amount of interrupts, all of the code was synchronous-blocking, and the scheduling of various modules' code was very rudimentary. The code was also rushed for the competition and never fully completed.

Rather than continuing with this version, I wanted to try redesign the code from the ground up using bleeding-edge embedded Rust frameworks. This will allow us to use a wider range of embedded boards, namely the recent Raspberry Pi Pico, which are not only much more powerful, but also much cheaper. It also gives me the opportunity to learn how the drivers were designed, what limitations exist, and what improvements can be made while I rewrite them from scratch.

While embedded Rust has a much steeper learning curve, it has many benefits. Besides low-level access to all of the hardware, there are powerful features such as ownership, thread-safety, and even realtime asynchronous frameworks. While this code can serve as a stepping stone to building our future competition robots using Rust, that isn't the main goal. I want to practice all the skills I'm learning in courses like EGRE-364 and EGRE-365 and apply them to something new, while being an educational reference for the rest of the power systems team.

# Dependency Overview
 
 - `probe-run` allows for running with debugging/semihosting using cargo
 - `flip-link` swaps the stack and data sectors to have stack overflow detection during runtime
 - `embedded_hal` creates traits for device-agnostic driver implementation
 - `rp_pico` is a board-access crate, mapping processor outputs to physical board pinouts and more
 - `cortex_m` allows in-place assembly and more
 - `cortex_m_rtic` is an implementation of the RTIC (Real-Time Interrupt-driven Concurrency) framework for cortex-m devices. This allows for "tasks" to run independently of each other while sharing state. It also includes a powerful scheduler, allowing for many software tasks to share the processor as well as interrupts.

# Running

Follow the steps [here](https://github.com/rp-rs/rp2040-project-template). Note that you'll need a debugger to run the code interactively, the easiest method is to use a second pico as a debugger.

# Wiring Guide (WIP)

Host Pi:

 - i-BUS input: GPIO0
 - i-BUS output: GPIO1

Debugger:

 - SWCLK: GPIO2
 - SWDIO: GPIO3
