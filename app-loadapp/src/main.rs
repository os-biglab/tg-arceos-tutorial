#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

#[macro_use]
#[cfg(feature = "axstd")]
extern crate axstd as std;

#[cfg(feature = "axstd")]
extern crate axfs;
#[cfg(feature = "axstd")]
extern crate axio;

#[cfg_attr(feature = "axstd", unsafe(no_mangle))]
fn main() {
    #[cfg(feature = "axstd")]
    {
        use axfs::ROOT_FS_CONTEXT;
        #[allow(unused_imports)]
        use axio::{Read, Write};
        use std::thread;

        println!("Load app from fat-fs ...");

        let mut buf = [0u8; 64];
        if let Err(e) = load_app("/sbin/origin.bin", &mut buf) {
            panic!("Cannot load app! {:?}", e);
        }

        let worker1 = thread::spawn(move || {
            println!("worker1 checks code: ");
            for i in 0..8 {
                print!("{:#x} ", buf[i]);
            }
            println!("\nworker1 ok!");
        });

        println!("Wait for workers to exit ...");
        let _ = worker1.join();

        println!("Load app from disk ok!");

        // Exercise: demonstrate rename functionality using axfs
        println!("\nRunning rename exercise ...");
        if let Err(e) = rename_exercise() {
            panic!("Rename exercise failed: {:?}", e);
        }
        println!("[Ramfs-Rename]: ok!");

        fn load_app(fname: &str, buf: &mut [u8]) -> Result<usize, axio::Error> {
            println!("fname: {}", fname);
            let ctx = ROOT_FS_CONTEXT.get().expect("Root FS not initialized");
            let file = axfs::File::open(ctx, fname)
                .map_err(|_| axio::Error::NotFound)?;
            let n = (&file).read(buf)?;
            Ok(n)
        }

        fn rename_exercise() -> Result<(), axio::Error> {
            use axfs_ng_vfs::NodePermission;

            let ctx = ROOT_FS_CONTEXT.get().expect("Root FS not initialized");

            // Create /tmp directory (ignore error if already exists)
            let _ = ctx.create_dir("/tmp", NodePermission::default());

            // Create file /tmp/f1 with content "hello"
            println!("Create '/tmp/f1' and write [hello] ...");
            {
                let file = axfs::File::create(ctx, "/tmp/f1")
                    .map_err(|_| axio::Error::AlreadyExists)?;
                (&file).write_all(b"hello")?;
            }

            // Rename /tmp/f1 to /tmp/f2 (same directory = rename, not move)
            println!("Rename '/tmp/f1' to '/tmp/f2' ...");
            ctx.rename("/tmp/f1", "/tmp/f2")
                .map_err(|_| axio::Error::NotFound)?;

            // Read back /tmp/f2
            let file = axfs::File::open(ctx, "/tmp/f2")
                .map_err(|_| axio::Error::NotFound)?;
            let mut buf = [0u8; 64];
            let n = (&file).read(&mut buf[..])?;
            let content = core::str::from_utf8(&buf[..n]).unwrap_or("(invalid utf8)");
            print!("Read '/tmp/f2' content: [");
            print!("{}", content);
            println!("] ok!");

            Ok(())
        }
    }
    #[cfg(not(feature = "axstd"))]
    {
        println!("This application requires the 'axstd' feature for filesystem access.");
        println!("Run with: cargo xtask run [--arch <ARCH>]");
    }
}
