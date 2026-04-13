//! Syscall handler for user-space Linux apps.
//!
//! Extended to support file I/O and mmap for the mapfile exercise.

use alloc::sync::Arc;
use alloc::vec::Vec;
use axhal::uspace::UserContext;
use axmm::AddrSpace;
use axsync::Mutex;

// ---- Architecture-specific syscall numbers ----

#[cfg(not(target_arch = "x86_64"))]
mod nums {
    pub const SYS_IOCTL: usize = 29;
    pub const SYS_OPENAT: usize = 56;
    pub const SYS_CLOSE: usize = 57;
    pub const SYS_READ: usize = 63;
    pub const SYS_WRITE: usize = 64;
    pub const SYS_WRITEV: usize = 66;
    pub const SYS_EXIT: usize = 93;
    pub const SYS_EXIT_GROUP: usize = 94;
    pub const SYS_SET_TID_ADDRESS: usize = 96;
    pub const SYS_GETUID: usize = 174;
    pub const SYS_GETEUID: usize = 175;
    pub const SYS_GETGID: usize = 176;
    pub const SYS_GETEGID: usize = 177;
    pub const SYS_BRK: usize = 214;
    pub const SYS_MMAP: usize = 222;
}

#[cfg(target_arch = "x86_64")]
mod nums {
    pub const SYS_READ: usize = 0;
    pub const SYS_WRITE: usize = 1;
    pub const SYS_CLOSE: usize = 3;
    pub const SYS_MMAP: usize = 9;
    pub const SYS_IOCTL: usize = 16;
    pub const SYS_WRITEV: usize = 20;
    pub const SYS_OPENAT: usize = 257;
    pub const SYS_EXIT: usize = 60;
    pub const SYS_EXIT_GROUP: usize = 231;
    pub const SYS_SET_TID_ADDRESS: usize = 218;
    pub const SYS_GETUID: usize = 102;
    pub const SYS_GETGID: usize = 104;
    pub const SYS_GETEUID: usize = 107;
    pub const SYS_GETEGID: usize = 108;
    pub const SYS_BRK: usize = 12;
    pub const SYS_ARCH_PRCTL: usize = 158;
    pub const ARCH_SET_FS: usize = 0x1002;
}

use nums::*;
use core::sync::atomic::{AtomicUsize, Ordering};

// AT_FDCWD: use current working directory
const AT_FDCWD: i32 = -100i32;

// O_RDONLY, O_WRONLY, O_CREAT, O_TRUNC flags
const O_RDONLY: i32 = 0;
const O_WRONLY: i32 = 1;
const O_CREAT: i32 = 0o100;
const O_TRUNC: i32 = 0o1000;

// mmap flags
const MAP_ANONYMOUS: i32 = 1 << 5;

// mmap prot flags
const PROT_READ: i32 = 1;
const PROT_WRITE: i32 = 2;
const PROT_EXEC: i32 = 4;

// ---- Simple file descriptor table ----
// We use a global fd table since the task owns the address space.
// FDs 0,1,2 are stdin/stdout/stderr (not real files).
// FDs >= 3 are real files opened via axfs.

use spin::Once;

static FD_TABLE: Once<Mutex<FdTable>> = Once::new();
static BRK_TOP: AtomicUsize = AtomicUsize::new(0x2000_0000);

fn fd_table() -> &'static Mutex<FdTable> {
    FD_TABLE.call_once(|| Mutex::new(FdTable::new()))
}

struct FdTable {
    entries: Vec<Option<FdEntry>>,
}

struct FdEntry {
    file: axfs::File,
    offset: u64,
}

impl FdTable {
    fn new() -> Self {
        // Pre-allocate slots 0,1,2 as None (stdin/stdout/stderr)
        Self {
            entries: alloc::vec![None, None, None],
        }
    }

    fn alloc(&mut self, file: axfs::File) -> i32 {
        // Find first empty slot >= 3
        for (i, slot) in self.entries.iter_mut().enumerate() {
            if slot.is_none() && i >= 3 {
                *slot = Some(FdEntry { file, offset: 0 });
                return i as i32;
            }
        }
        // Append new slot
        let fd = self.entries.len() as i32;
        self.entries.push(Some(FdEntry { file, offset: 0 }));
        fd
    }

    fn get_offset(&self, fd: i32) -> Option<u64> {
        self.entries.get(fd as usize)?.as_ref().map(|e| e.offset)
    }

    fn set_offset(&mut self, fd: i32, offset: u64) {
        if let Some(Some(entry)) = self.entries.get_mut(fd as usize) {
            entry.offset = offset;
        }
    }

    fn close(&mut self, fd: i32) -> bool {
        if let Some(slot) = self.entries.get_mut(fd as usize) {
            if slot.is_some() {
                *slot = None;
                return true;
            }
        }
        false
    }

    /// Read file content at current offset, advance offset.
    fn read(&mut self, fd: i32, buf: &mut [u8]) -> isize {
        let entry = match self.entries.get_mut(fd as usize).and_then(|s| s.as_mut()) {
            Some(e) => e,
            None => return -(axerrno::LinuxError::EBADF.code() as isize),
        };
        use axio::Read;
        match (&entry.file).read(buf) {
            Ok(n) => {
                entry.offset += n as u64;
                n as isize
            }
            Err(_) => -(axerrno::LinuxError::EIO.code() as isize),
        }
    }

    /// Write to file at current offset, advance offset.
    fn write(&mut self, fd: i32, buf: &[u8]) -> isize {
        let entry = match self.entries.get_mut(fd as usize).and_then(|s| s.as_mut()) {
            Some(e) => e,
            None => return -(axerrno::LinuxError::EBADF.code() as isize),
        };
        use axio::Write;
        match (&entry.file).write(buf) {
            Ok(n) => {
                entry.offset += n as u64;
                n as isize
            }
            Err(_) => -(axerrno::LinuxError::EIO.code() as isize),
        }
    }

    /// Read entire file content from beginning (for mmap).
    fn read_all(&self, fd: i32) -> Option<Vec<u8>> {
        let entry = self.entries.get(fd as usize)?.as_ref()?;
        use axio::{Read, Seek, SeekFrom};
        // Seek to beginning
        let _ = (&entry.file).seek(SeekFrom::Start(0));
        let mut data = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            match (&entry.file).read(&mut buf[..]) {
                Ok(0) => break,
                Ok(n) => data.extend_from_slice(&buf[..n]),
                Err(_) => break,
            }
        }
        Some(data)
    }
}

// ---- iovec structure for writev ----

#[repr(C)]
struct IoVec {
    iov_base: usize,
    iov_len: usize,
}

// ---- Architecture-specific register access ----

#[cfg(any(target_arch = "riscv64", target_arch = "riscv32", target_arch = "loongarch64"))]
fn get_syscall_num(uctx: &UserContext) -> usize {
    uctx.regs.a7
}

#[cfg(target_arch = "aarch64")]
fn get_syscall_num(uctx: &UserContext) -> usize {
    uctx.x[8] as usize
}

#[cfg(target_arch = "x86_64")]
fn get_syscall_num(uctx: &UserContext) -> usize {
    uctx.rax as usize
}

fn set_syscall_ret(uctx: &mut UserContext, ret: usize) {
    #[cfg(target_arch = "x86_64")]
    {
        uctx.rax = ret as u64;
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        uctx.set_arg0(ret);
    }
}

/// Handle a syscall from user space.
///
/// Returns `Some(exit_code)` if the process should exit, `None` to continue.
pub fn handle_syscall(uctx: &mut UserContext, uspace: &mut AddrSpace) -> Option<i32> {
    let syscall_num = get_syscall_num(uctx);
    ax_println!("handle_syscall [{}] ...", syscall_num);

    let ret: isize = match syscall_num {
        SYS_IOCTL => {
            ax_println!("Ignore SYS_IOCTL");
            0
        }
        SYS_SET_TID_ADDRESS => {
            axtask::current().id().as_u64() as isize
        }
        SYS_GETUID | SYS_GETEUID | SYS_GETGID | SYS_GETEGID => 0,
        SYS_BRK => {
            let new_brk = uctx.arg0();
            sys_brk(new_brk, uspace)
        }
        SYS_OPENAT => {
            let dfd = uctx.arg0() as i32;
            let fname_ptr = uctx.arg1() as *const u8;
            let flags = uctx.arg2() as i32;
            sys_openat(dfd, fname_ptr, flags, uspace)
        }
        SYS_CLOSE => {
            let fd = uctx.arg0() as i32;
            sys_close(fd)
        }
        SYS_READ => {
            let fd = uctx.arg0() as i32;
            let buf_ptr = uctx.arg1() as usize;
            let count = uctx.arg2();
            sys_read(fd, buf_ptr, count, uspace)
        }
        SYS_WRITE => {
            let fd = uctx.arg0() as i32;
            let buf_ptr = uctx.arg1() as usize;
            let count = uctx.arg2();
            sys_write_fd(fd, buf_ptr, count, uspace)
        }
        SYS_WRITEV => {
            let fd = uctx.arg0() as i32;
            let iov_ptr = uctx.arg1() as *const IoVec;
            let iovcnt = uctx.arg2() as i32;
            sys_writev(fd, iov_ptr, iovcnt)
        }
        SYS_MMAP => {
            let addr = uctx.arg0();
            let length = uctx.arg1();
            let prot = uctx.arg2() as i32;
            let flags = uctx.arg3() as i32;
            let fd = uctx.arg4() as i32;
            let offset = uctx.arg5() as i64;
            sys_mmap(addr, length, prot, flags, fd, offset, uspace)
        }
        SYS_EXIT_GROUP => {
            ax_println!("[SYS_EXIT_GROUP]: system is exiting ..");
            return Some(uctx.arg0() as i32);
        }
        SYS_EXIT => {
            ax_println!("[SYS_EXIT]: system is exiting ..");
            return Some(uctx.arg0() as i32);
        }
        #[cfg(target_arch = "x86_64")]
        SYS_ARCH_PRCTL => {
            let op = uctx.arg0();
            let addr = uctx.arg1();
            sys_arch_prctl(uctx, op, addr)
        }
        _ => {
            ax_println!("Unimplemented syscall: {}", syscall_num);
            -(axerrno::LinuxError::ENOSYS.code() as isize)
        }
    };

    set_syscall_ret(uctx, ret as usize);
    None
}

/// Implement a minimal brk heap for libc startup.
fn sys_brk(new_brk: usize, uspace: &mut AddrSpace) -> isize {
    use axhal::paging::{MappingFlags, PageSize};
    use axmm::backend::{Backend, SharedPages};
    use memory_addr::VirtAddr;

    let cur = BRK_TOP.load(Ordering::SeqCst);
    if new_brk == 0 {
        return cur as isize;
    }

    if new_brk <= cur {
        BRK_TOP.store(new_brk, Ordering::SeqCst);
        return new_brk as isize;
    }

    let page_size = 0x1000usize;
    let old_aligned = (cur + page_size - 1) & !(page_size - 1);
    let new_aligned = (new_brk + page_size - 1) & !(page_size - 1);

    if new_aligned > old_aligned {
        let map_size = new_aligned - old_aligned;
        let vaddr = VirtAddr::from(old_aligned);
        let pages = match SharedPages::new(map_size, PageSize::Size4K) {
            Ok(p) => Arc::new(p),
            Err(_) => return cur as isize,
        };
        if uspace
            .map(
                vaddr,
                map_size,
                MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
                true,
                Backend::new_shared(vaddr, pages),
            )
            .is_err()
        {
            return cur as isize;
        }
    }

    BRK_TOP.store(new_brk, Ordering::SeqCst);
    new_brk as isize
}

/// Read a null-terminated C string from user space.
fn read_cstr_from_uspace(ptr: *const u8, uspace: &AddrSpace) -> alloc::string::String {
    let mut result = Vec::new();
    let mut addr = ptr as usize;
    loop {
        let mut byte = [0u8; 1];
        if uspace.read(addr.into(), &mut byte).is_err() {
            break;
        }
        if byte[0] == 0 {
            break;
        }
        result.push(byte[0]);
        addr += 1;
        if result.len() > 4096 {
            break;
        }
    }
    alloc::string::String::from_utf8_lossy(&result).into_owned()
}

/// Open a file. Handles both create and open modes.
fn sys_openat(dfd: i32, fname_ptr: *const u8, flags: i32, uspace: &AddrSpace) -> isize {
    let raw_fname = read_cstr_from_uspace(fname_ptr, uspace);
    let fname = if raw_fname.starts_with('/') {
        raw_fname
    } else {
        alloc::format!("/{raw_fname}")
    };
    ax_println!("sys_openat: dfd={}, fname={:?}, flags={:#o}", dfd, fname, flags);

    let ctx = match axfs::ROOT_FS_CONTEXT.get() {
        Some(c) => c,
        None => return -(axerrno::LinuxError::ENOENT.code() as isize),
    };

    let file = if flags & O_CREAT != 0 {
        // Create or truncate
        axfs::File::create(ctx, fname.as_str())
    } else {
        // Open existing
        axfs::File::open(ctx, fname.as_str())
    };

    match file {
        Ok(f) => {
            let fd = fd_table().lock().alloc(f);
            ax_println!("sys_openat: opened fd={}", fd);
            fd as isize
        }
        Err(e) => {
            ax_println!("sys_openat: error {:?}", e);
            -(axerrno::LinuxError::ENOENT.code() as isize)
        }
    }
}

/// Close a file descriptor.
fn sys_close(fd: i32) -> isize {
    if fd_table().lock().close(fd) {
        0
    } else {
        -(axerrno::LinuxError::EBADF.code() as isize)
    }
}

/// Read from a file descriptor into user-space buffer.
fn sys_read(fd: i32, buf_ptr: usize, count: usize, uspace: &mut AddrSpace) -> isize {
    let mut buf = alloc::vec![0u8; count];
    let n = fd_table().lock().read(fd, &mut buf);
    if n > 0 {
        if uspace.write(buf_ptr.into(), &buf[..n as usize]).is_err() {
            return -(axerrno::LinuxError::EFAULT.code() as isize);
        }
    }
    n
}

/// Write from user-space buffer to a file descriptor.
fn sys_write_fd(fd: i32, buf_ptr: usize, count: usize, uspace: &AddrSpace) -> isize {
    // For stdout/stderr, use writev-style output
    if fd == 1 || fd == 2 {
        let mut buf = alloc::vec![0u8; count];
        if uspace.read(buf_ptr.into(), &mut buf).is_err() {
            return -(axerrno::LinuxError::EFAULT.code() as isize);
        }
        for &b in &buf {
            ax_print!("{}", b as char);
        }
        return count as isize;
    }

    let mut buf = alloc::vec![0u8; count];
    if uspace.read(buf_ptr.into(), &mut buf).is_err() {
        return -(axerrno::LinuxError::EFAULT.code() as isize);
    }
    fd_table().lock().write(fd, &buf)
}

/// Write data from an iovec array to a file descriptor.
fn sys_writev(fd: i32, iov: *const IoVec, iovcnt: i32) -> isize {
    if fd != 1 && fd != 2 {
        return -(axerrno::LinuxError::EBADF.code() as isize);
    }

    let mut total: isize = 0;
    for i in 0..iovcnt as usize {
        let entry = unsafe { &*iov.add(i) };
        if entry.iov_len == 0 || entry.iov_base == 0 {
            continue;
        }
        let slice =
            unsafe { core::slice::from_raw_parts(entry.iov_base as *const u8, entry.iov_len) };
        for &b in slice {
            ax_print!("{}", b as char);
        }
        total += entry.iov_len as isize;
    }
    total
}

/// Implement sys_mmap: map memory or file into user address space.
fn sys_mmap(
    addr: usize,
    length: usize,
    prot: i32,
    flags: i32,
    fd: i32,
    offset: i64,
    uspace: &mut AddrSpace,
) -> isize {
    use axhal::paging::{MappingFlags, PageSize};
    use axmm::backend::{Backend, SharedPages};
    use memory_addr::{MemoryAddr, VirtAddr};

    ax_println!(
        "sys_mmap: addr={:#x}, len={:#x}, prot={:#x}, flags={:#x}, fd={}, offset={}",
        addr, length, prot, flags, fd, offset
    );

    if length == 0 {
        return -(axerrno::LinuxError::EINVAL.code() as isize);
    }

    // Round up length to page size
    let page_size = 0x1000usize;
    let map_size = (length + page_size - 1) & !(page_size - 1);

    // Determine mapping address: if addr == 0, find a free region
    let map_vaddr = if addr == 0 {
        // Find a free region in user space (simple bump: use a fixed region)
        // Use a region below the stack, above the ELF segments
        // We'll use 0x1000_0000 as a base for mmap allocations
        static MMAP_BASE: core::sync::atomic::AtomicUsize =
            core::sync::atomic::AtomicUsize::new(0x1000_0000);
        let va = MMAP_BASE.fetch_add(map_size, core::sync::atomic::Ordering::SeqCst);
        va
    } else {
        addr
    };

    let vaddr = VirtAddr::from(map_vaddr);

    // Build mapping flags from prot
    let mut mapping_flags = MappingFlags::USER;
    if prot & PROT_READ != 0 {
        mapping_flags |= MappingFlags::READ;
    }
    if prot & PROT_WRITE != 0 {
        mapping_flags |= MappingFlags::WRITE;
    }
    if prot & PROT_EXEC != 0 {
        mapping_flags |= MappingFlags::EXECUTE;
    }

    // Allocate physical pages
    let pages = match SharedPages::new(map_size, PageSize::Size4K) {
        Ok(p) => Arc::new(p),
        Err(_) => return -(axerrno::LinuxError::ENOMEM.code() as isize),
    };

    // Map the pages into user address space
    if uspace
        .map(
            vaddr,
            map_size,
            mapping_flags,
            true,
            Backend::new_shared(vaddr, pages),
        )
        .is_err()
    {
        return -(axerrno::LinuxError::ENOMEM.code() as isize);
    }

    // If file-backed mapping, read file content into the mapped region
    if flags & MAP_ANONYMOUS == 0 && fd >= 0 {
        // Read file content
        let file_data = match fd_table().lock().read_all(fd) {
            Some(data) => data,
            None => return -(axerrno::LinuxError::EBADF.code() as isize),
        };

        let file_offset = offset as usize;
        let copy_len = length.min(file_data.len().saturating_sub(file_offset));

        if copy_len > 0 {
            let src = &file_data[file_offset..file_offset + copy_len];
            ax_println!("sys_mmap: copying {} bytes from file to {:#x}", copy_len, map_vaddr);
            if uspace.write(vaddr, src).is_err() {
                return -(axerrno::LinuxError::EFAULT.code() as isize);
            }
        }
    }

    ax_println!("sys_mmap: mapped at {:#x}", map_vaddr);
    map_vaddr as isize
}

/// Handle x86_64 arch_prctl for TLS setup.
#[cfg(target_arch = "x86_64")]
fn sys_arch_prctl(uctx: &mut UserContext, op: usize, addr: usize) -> isize {
    use nums::{ARCH_SET_FS, SYS_ARCH_PRCTL};
    match op {
        ARCH_SET_FS => {
            uctx.fs_base = addr as u64;
            0
        }
        _ => {
            ax_println!("Unimplemented arch_prctl op: {:#x}", op);
            -(axerrno::LinuxError::EINVAL.code() as isize)
        }
    }
}
