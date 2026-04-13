//! User task spawning and trap dispatch loop.

use axhal::uspace::{ReturnReason, UserContext};
use axmm::AddrSpace;
use axtask::{AxTaskRef, TaskInner};
use memory_addr::VirtAddr;

use crate::syscall;

#[repr(C, align(16))]
struct AlignedUserContext(UserContext);

/// Spawn a user task that enters user space and handles traps.
pub fn spawn_user_task(
    uspace: AddrSpace,
    entry: usize,
    ustack_pointer: VirtAddr,
) -> AxTaskRef {
    let page_table_root = uspace.page_table_root();

    let mut task = TaskInner::new(
        move || {
            // Keep uspace alive and mutable for syscall handling (e.g. mmap).
            let mut uspace = uspace;

            let mut aligned_uctx =
                AlignedUserContext(UserContext::new(entry, ustack_pointer, 0));

            #[cfg(target_arch = "aarch64")]
            axhal::asm::enable_fp();

            ax_println!(
                "Enter user space: entry={:#x}, ustack={:#x}, kstack={:#x}",
                entry,
                ustack_pointer,
                axtask::current().kernel_stack_top().unwrap(),
            );

            loop {
                let reason = aligned_uctx.0.run();
                match reason {
                    ReturnReason::Syscall => {
                        if let Some(exit_code) =
                            syscall::handle_syscall(&mut aligned_uctx.0, &mut uspace)
                        {
                            axtask::exit(exit_code as _);
                        }
                    }
                    ReturnReason::Interrupt => {
                        // Timer interrupt — re-enter user space.
                    }
                    _ => {
                        ax_println!("Unexpected trap from user space: {:?}", reason);
                        axtask::exit(-1);
                    }
                }
            }
        },
        "userboot".into(),
        crate::KERNEL_STACK_SIZE,
    );

    task.ctx_mut().set_page_table_root(page_table_root);
    axtask::spawn_task(task)
}
