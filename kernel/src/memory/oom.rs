use crate::memory::pmm::Pmm;

/// OOM recovery levels.
/// Level 1: Shrink block cache
/// Level 2: Flush dirty blocks
/// Level 3: Kill largest process (not init)
pub fn try_recover() -> bool {
    let pmm = Pmm::get();
    let free = pmm.free_frames();

    // Level 1: Shrink block cache if > 0 entries
    let (cache_entries, _, _) = crate::fs::block_cache::stats();
    if cache_entries > 0 {
        let _ = crate::fs::block_cache::sync();
        crate::serial_println!(
            "OOM: Level 1 — synced block cache ({} entries)",
            cache_entries
        );
        let new_free = pmm.free_frames();
        if new_free > free + 10 {
            return true;
        }
    }

    // Level 2: Nothing more to reclaim without killing processes
    crate::serial_println!("OOM: Level 2 — no reclaimable memory");

    // Level 3: Would kill largest non-init process, but we don't have
    // real multi-process yet. Log and fail.
    crate::serial_println!("OOM: Level 3 — no killable process (single-process kernel)");

    false
}

/// Check if memory is critically low (< 5% free).
pub fn is_low() -> bool {
    let pmm = Pmm::get();
    let total = pmm.total_frames();
    let free = pmm.free_frames();
    free < total / 20
}
