#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

#[cfg(feature = "axstd")]
extern crate alloc;

#[cfg(feature = "axstd")]
#[macro_use]
extern crate axstd as std;

#[cfg(feature = "axstd")]
use alloc::string::String;
#[cfg(feature = "axstd")]
use alloc::vec;

// Use hashbrown for HashMap in no_std environment
use hashbrown::HashMap;

#[cfg_attr(feature = "axstd", unsafe(no_mangle))]
fn main() {
    let s = String::from("Hello, axalloc!");
    println!("Alloc String: \"{}\"", s);

    let mut v = vec![0, 1, 2];
    v.push(3);
    println!("Alloc Vec: {:?}", v);

    println!("Running memory tests...");
    test_hashmap();
    println!("Memory tests run OK!");
}

fn test_hashmap() {
    const N: u32 = 50_000;
    let mut m = HashMap::new();
    for value in 0..N {
        let key = alloc::format!("key_{value}");
        m.insert(key, value);
    }
    for (k, v) in m.iter() {
        if let Some(k) = k.strip_prefix("key_") {
            assert_eq!(k.parse::<u32>().unwrap(), *v);
        }
    }
    println!("test_hashmap() OK!");
}
