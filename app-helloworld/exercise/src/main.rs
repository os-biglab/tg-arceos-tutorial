#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

#[cfg(feature = "axstd")]
use axstd::println;

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    // ANSI escape sequence makes the target message colored on terminal consoles.
    println!("\x1b[32mHello, \x1b[31mworld!\x1b[0m");
}
