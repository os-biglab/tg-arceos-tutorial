use alloc::sync::Arc;
use alloc::vec;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};

use axhal::paging::{MappingFlags, PageSize};
use axhal::mem::{MemoryAddr, VirtAddr};
use axmm::backend::{Backend, SharedPages};
use axmm::AddrSpace;

const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];
const PT_LOAD: u32 = 1;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Elf64Ehdr {
    e_ident: [u8; 16],
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    e_entry: u64,
    e_phoff: u64,
    e_shoff: u64,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Elf64Phdr {
    p_type: u32,
    p_flags: u32,
    p_offset: u64,
    p_vaddr: u64,
    p_paddr: u64,
    p_filesz: u64,
    p_memsz: u64,
    p_align: u64,
}

const EHDR_SIZE: usize = core::mem::size_of::<Elf64Ehdr>();
const PHDR_SIZE: usize = core::mem::size_of::<Elf64Phdr>();

fn read_exact(file: &mut File, buf: &mut [u8]) -> io::Result<()> {
    let mut offset = 0;
    while offset < buf.len() {
        let n = file.read(&mut buf[offset..])?;
        if n == 0 {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
        }
        offset += n;
    }
    Ok(())
}

pub fn load_user_app(fname: &str, uspace: &mut AddrSpace) -> io::Result<usize> {
    let mut file = File::open(fname)?;

    let mut ehdr_buf = [0u8; EHDR_SIZE];
    read_exact(&mut file, &mut ehdr_buf)?;
    let ehdr: Elf64Ehdr = unsafe { core::ptr::read_unaligned(ehdr_buf.as_ptr() as *const _) };

    if ehdr.e_ident[..4] != ELF_MAGIC {
        return Err(io::Error::from(io::ErrorKind::InvalidData));
    }

    let entry = ehdr.e_entry as usize;
    let phnum = ehdr.e_phnum as usize;
    let phoff = ehdr.e_phoff;

    let phdr_total = phnum * PHDR_SIZE;
    let mut phdr_buf = vec![0u8; phdr_total];
    file.seek(SeekFrom::Start(phoff))?;
    read_exact(&mut file, &mut phdr_buf)?;

    for i in 0..phnum {
        let phdr: Elf64Phdr = unsafe {
            core::ptr::read_unaligned(phdr_buf.as_ptr().add(i * PHDR_SIZE) as *const _)
        };
        if phdr.p_type != PT_LOAD {
            continue;
        }

        let vaddr = VirtAddr::from(phdr.p_vaddr as usize).align_down_4k();
        let vaddr_end = VirtAddr::from((phdr.p_vaddr + phdr.p_memsz) as usize).align_up_4k();
        let map_size = vaddr_end - vaddr;

        let pages = Arc::new(
            SharedPages::new(map_size, PageSize::Size4K)
                .map_err(|_| io::Error::from(io::ErrorKind::OutOfMemory))?,
        );
        uspace
            .map(
                vaddr,
                map_size,
                MappingFlags::READ | MappingFlags::WRITE | MappingFlags::EXECUTE | MappingFlags::USER,
                true,
                Backend::new_shared(vaddr, pages),
            )
            .map_err(|_| io::Error::from(io::ErrorKind::OutOfMemory))?;

        let mut data = vec![0u8; phdr.p_memsz as usize];
        file.seek(SeekFrom::Start(phdr.p_offset))?;
        let filesz = phdr.p_filesz as usize;
        let mut index = 0;
        while index < filesz {
            let n = file.read(&mut data[index..filesz])?;
            if n == 0 {
                break;
            }
            index += n;
        }

        uspace.write(VirtAddr::from(phdr.p_vaddr as usize), &data)?;
    }

    Ok(entry)
}
