#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

#[cfg(feature = "axstd")]
use axstd::println;

// ANSI color escape codes
const COLOR_RED: &str = "\x1b[31m";
const COLOR_GREEN: &str = "\x1b[32m";
const COLOR_RESET: &str = "\x1b[0m";

#[cfg_attr(feature = "axstd", unsafe(no_mangle))]
fn main() {
    println!("{}Hello, world!{}", COLOR_GREEN, COLOR_RESET);
    println!("{}[WithColor]: Hello, Arceos!{}", COLOR_RED, COLOR_RESET);
}