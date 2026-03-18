extern crate alloc;

use crate::syscall::errno::Errno;

/// ELF64 magic number.
const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];

/// ELF class.
const ELFCLASS64: u8 = 2;
/// ELF data encoding.
const ELFDATA2LSB: u8 = 1;
/// ELF type: executable.
const ET_EXEC: u16 = 2;
/// ELF machine: x86_64.
const EM_X86_64: u16 = 62;
/// Program header type: loadable segment.
const PT_LOAD: u32 = 1;

/// ELF64 file header.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Header {
    pub e_ident: [u8; 16],
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    pub e_entry: u64,
    pub e_phoff: u64,
    pub e_shoff: u64,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    pub e_phnum: u16,
    pub e_shentsize: u16,
    pub e_shnum: u16,
    pub e_shstrndx: u16,
}

/// ELF64 program header.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Phdr {
    pub p_type: u32,
    pub p_flags: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_paddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_align: u64,
}

/// Program header flags.
pub const PF_X: u32 = 1;
pub const PF_W: u32 = 2;
pub const PF_R: u32 = 4;

/// Parsed ELF information ready for loading.
#[derive(Debug)]
pub struct ElfInfo {
    pub entry_point: u64,
    pub segments: alloc::vec::Vec<ElfSegment>,
}

/// A loadable ELF segment.
#[derive(Debug, Clone)]
pub struct ElfSegment {
    pub vaddr: u64,
    pub memsz: u64,
    pub filesz: u64,
    pub offset: u64,
    pub flags: u32,
}

impl ElfSegment {
    pub fn is_executable(&self) -> bool {
        self.flags & PF_X != 0
    }
    pub fn is_writable(&self) -> bool {
        self.flags & PF_W != 0
    }
}

/// Validate and parse an ELF64 executable from a byte buffer.
pub fn parse(data: &[u8]) -> Result<ElfInfo, Errno> {
    if data.len() < core::mem::size_of::<Elf64Header>() {
        return Err(Errno::ENOEXEC);
    }

    // SAFETY: We verified the buffer is large enough for the header.
    let header: Elf64Header = unsafe { core::ptr::read(data.as_ptr() as *const Elf64Header) };

    // Validate magic
    if header.e_ident[..4] != ELF_MAGIC {
        return Err(Errno::ENOEXEC);
    }

    // Must be 64-bit, little-endian, executable, x86_64
    if header.e_ident[4] != ELFCLASS64 {
        return Err(Errno::ENOEXEC);
    }
    if header.e_ident[5] != ELFDATA2LSB {
        return Err(Errno::ENOEXEC);
    }
    if header.e_type != ET_EXEC {
        return Err(Errno::ENOEXEC);
    }
    if header.e_machine != EM_X86_64 {
        return Err(Errno::ENOEXEC);
    }

    // Parse program headers
    let phoff = header.e_phoff as usize;
    let phentsize = header.e_phentsize as usize;
    let phnum = header.e_phnum as usize;

    if phentsize < core::mem::size_of::<Elf64Phdr>() {
        return Err(Errno::ENOEXEC);
    }

    let mut segments = alloc::vec::Vec::new();

    for i in 0..phnum {
        let offset = phoff + i * phentsize;
        if offset + core::mem::size_of::<Elf64Phdr>() > data.len() {
            return Err(Errno::ENOEXEC);
        }

        let ptr = data[offset..].as_ptr() as *const Elf64Phdr;
        // SAFETY: Offset is within bounds and buffer is large enough for Elf64Phdr.
        let phdr: Elf64Phdr = unsafe { core::ptr::read(ptr) };

        if phdr.p_type != PT_LOAD {
            continue;
        }

        // Validate segment bounds within the file
        if phdr.p_offset + phdr.p_filesz > data.len() as u64 {
            return Err(Errno::ENOEXEC);
        }

        // W^X enforcement: a segment cannot be both writable and executable
        if phdr.p_flags & PF_W != 0 && phdr.p_flags & PF_X != 0 {
            return Err(Errno::ENOEXEC);
        }

        // Segment must be in user space (below kernel)
        if phdr.p_vaddr >= 0xFFFF_8000_0000_0000 {
            return Err(Errno::ENOEXEC);
        }

        segments.push(ElfSegment {
            vaddr: phdr.p_vaddr,
            memsz: phdr.p_memsz,
            filesz: phdr.p_filesz,
            offset: phdr.p_offset,
            flags: phdr.p_flags,
        });
    }

    if segments.is_empty() {
        return Err(Errno::ENOEXEC);
    }

    // Entry point must be in user space
    if header.e_entry >= 0xFFFF_8000_0000_0000 {
        return Err(Errno::ENOEXEC);
    }

    Ok(ElfInfo {
        entry_point: header.e_entry,
        segments,
    })
}
