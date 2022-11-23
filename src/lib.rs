#![no_std]
#![no_main]

// global logger
pub use defmt_rtt as _;

// panic handler
pub use panic_probe as _;

pub mod fs_ia6b_driver;
