#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

#[macro_use]
#[cfg(feature = "axstd")]
extern crate axstd as std;

#[cfg(feature = "axstd")]
mod ramfs;

use std::io::{self, prelude::*};
use std::fs::{self, File};

fn create_file(fname: &str, text: &str) -> io::Result<()> {
    println!("Create '{}' and write [{}] ...", fname, text);
    let mut file = File::create(fname)?;
    file.write_all(text.as_bytes())
}

// Only support rename, NOT move.
fn rename_file(src: &str, dst: &str) -> io::Result<()> {
    println!("Rename '{}' to '{}' ...", src, dst);
    fs::rename(src, dst)
}

fn print_file(fname: &str) -> io::Result<()> {
    let mut buf = [0; 1024];
    let mut file = File::open(fname)?;
    loop {
        let n = file.read(&mut buf)?;
        if n > 0 {
            print!("Read '{}' content: [", fname);
            io::stdout().write_all(&buf[..n])?;
            println!("] ok!");
        } else {
            return Ok(());
        }
    }
}

fn process() -> io::Result<()> {
    // Cleanup from previous runs to keep this exercise repeatable.
    let _ = fs::remove_file("/tmp/a.txt");
    let _ = fs::remove_file("/tmp/b.txt");
    let _ = fs::remove_file("/tmp/dirb/b.txt");
    let _ = fs::remove_dir("/tmp/dira");
    let _ = fs::remove_dir("/tmp/dirb");

    // 1) directory rename
    println!("Create '/tmp/dira' ...");
    fs::create_dir("/tmp/dira")?;
    rename_file("/tmp/dira", "/tmp/dirb")?;

    // 2) file rename in same directory
    create_file("/tmp/a.txt", "hello")?;
    rename_file("/tmp/a.txt", "/tmp/b.txt")?;

    // 3) move file into another directory (mv)
    rename_file("/tmp/b.txt", "/tmp/dirb/b.txt")?;
    print_file("/tmp/dirb/b.txt")
}

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    if let Err(e) = process() {
        panic!("Error: {}", e);
    }
    println!("\n[Ramfs-Rename]: ok!");
}
