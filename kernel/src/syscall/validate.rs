extern crate alloc;

use crate::syscall::errno::Errno;

/// Minimum kernel virtual address — anything at or above this is kernel space.
const KERNEL_SPACE_START: u64 = 0xFFFF_8000_0000_0000;

/// Validate that a user pointer does not point into kernel space.
pub fn validate_user_ptr(ptr: u64, len: u64) -> Result<(), Errno> {
    if ptr == 0 {
        return Err(Errno::EFAULT);
    }

    // Check that the entire range is below kernel space
    let end = ptr.checked_add(len).ok_or(Errno::EFAULT)?;
    if ptr >= KERNEL_SPACE_START || end > KERNEL_SPACE_START {
        return Err(Errno::EFAULT);
    }

    Ok(())
}

/// Validate and copy a user-space string into a kernel buffer.
///
/// Copies byte-by-byte to avoid TOCTOU issues. Returns the number of
/// bytes copied (not including the null terminator).
pub fn copy_string_from_user(
    user_ptr: u64,
    buf: &mut [u8],
    max_len: usize,
) -> Result<usize, Errno> {
    validate_user_ptr(user_ptr, 1)?;

    let limit = buf.len().min(max_len);
    let src = user_ptr as *const u8;

    #[allow(clippy::needless_range_loop)]
    for i in 0..limit {
        // SAFETY: We validated the pointer is in user space. Each byte is
        // read individually to prevent TOCTOU race conditions.
        let byte = unsafe { core::ptr::read_volatile(src.add(i)) };
        if byte == 0 {
            return Ok(i);
        }
        // Re-validate that we haven't crossed into kernel space
        if user_ptr + i as u64 + 1 >= KERNEL_SPACE_START {
            return Err(Errno::EFAULT);
        }
        buf[i] = byte;
    }

    Err(Errno::ENAMETOOLONG)
}

/// Copy bytes from user space into a kernel buffer.
pub fn copy_from_user(user_ptr: u64, buf: &mut [u8]) -> Result<(), Errno> {
    validate_user_ptr(user_ptr, buf.len() as u64)?;

    let src = user_ptr as *const u8;
    for (i, byte) in buf.iter_mut().enumerate() {
        // SAFETY: Pointer validated above, reading byte-by-byte for safety.
        *byte = unsafe { core::ptr::read_volatile(src.add(i)) };
    }
    Ok(())
}

/// Copy a null-terminated string from user space into a kernel String.
pub fn copy_str_from_user(user_ptr: u64, max_len: usize) -> Result<alloc::string::String, Errno> {
    let mut buf = alloc::vec![0u8; max_len];
    let len = copy_string_from_user(user_ptr, &mut buf, max_len)?;
    alloc::string::String::from_utf8(buf[..len].to_vec()).map_err(|_| Errno::EINVAL)
}

/// Copy bytes from a kernel buffer to user space.
pub fn copy_to_user(user_ptr: u64, data: &[u8]) -> Result<(), Errno> {
    validate_user_ptr(user_ptr, data.len() as u64)?;

    let dst = user_ptr as *mut u8;
    for (i, &byte) in data.iter().enumerate() {
        // SAFETY: Pointer validated above, writing byte-by-byte for safety.
        unsafe { core::ptr::write_volatile(dst.add(i), byte) };
    }
    Ok(())
}
