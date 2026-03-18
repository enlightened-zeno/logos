extern crate alloc;

use crate::sync::SpinLock;
use alloc::collections::BTreeMap;
use alloc::vec;
use alloc::vec::Vec;

/// Block write function type.
type BlockWriteFn = fn(u64, &[u8]) -> Result<(), &'static str>;

/// LRU write-back block cache.
///
/// Caches disk blocks in memory. Dirty blocks are written back on eviction
/// or explicit sync. Capacity is capped at min(10% RAM, 16 MiB).
const MAX_CACHE_BYTES: usize = 16 * 1024 * 1024; // 16 MiB

struct CacheEntry {
    data: Vec<u8>,
    dirty: bool,
    /// LRU counter — higher is more recent.
    access_count: u64,
}

struct CacheInner {
    entries: BTreeMap<u64, CacheEntry>,
    block_size: usize,
    total_bytes: usize,
    max_bytes: usize,
    access_counter: u64,
    write_fn: Option<BlockWriteFn>,
}

static CACHE: SpinLock<Option<CacheInner>> = SpinLock::new(None);

/// Initialize the block cache.
pub fn init(block_size: usize, write_fn: BlockWriteFn) {
    let pmm = crate::memory::pmm::Pmm::get();
    let ram_bytes = pmm.total_frames() as usize * 4096;
    let max_bytes = (ram_bytes / 10).min(MAX_CACHE_BYTES);

    *CACHE.lock() = Some(CacheInner {
        entries: BTreeMap::new(),
        block_size,
        total_bytes: 0,
        max_bytes,
        access_counter: 0,
        write_fn: Some(write_fn),
    });

    crate::serial_println!(
        "Block cache: {} KiB max, block_size={}",
        max_bytes / 1024,
        block_size
    );
}

/// Read a block, returning cached data if available.
pub fn read(
    block: u64,
    read_fn: fn(u64, &mut [u8]) -> Result<(), &'static str>,
) -> Result<Vec<u8>, &'static str> {
    let mut guard = CACHE.lock();
    let cache = guard.as_mut().ok_or("Block cache not initialized")?;

    cache.access_counter += 1;
    let counter = cache.access_counter;

    if let Some(entry) = cache.entries.get_mut(&block) {
        entry.access_count = counter;
        return Ok(entry.data.clone());
    }

    // Cache miss — read from disk
    let mut data = vec![0u8; cache.block_size];
    read_fn(block, &mut data)?;

    // Evict if needed
    while cache.total_bytes + cache.block_size > cache.max_bytes {
        evict_lru(cache);
    }

    let bs = cache.block_size;
    cache.entries.insert(
        block,
        CacheEntry {
            data: data.clone(),
            dirty: false,
            access_count: counter,
        },
    );
    cache.total_bytes += bs;

    Ok(data)
}

/// Write a block through the cache.
pub fn write(block: u64, data: &[u8]) -> Result<(), &'static str> {
    let mut guard = CACHE.lock();
    let cache = guard.as_mut().ok_or("Block cache not initialized")?;

    cache.access_counter += 1;
    let counter = cache.access_counter;
    let bs = cache.block_size;

    if let Some(entry) = cache.entries.get_mut(&block) {
        entry.data[..data.len()].copy_from_slice(data);
        entry.dirty = true;
        entry.access_count = counter;
        return Ok(());
    }

    // New entry
    while cache.total_bytes + bs > cache.max_bytes {
        evict_lru(cache);
    }

    let mut full_data = vec![0u8; bs];
    full_data[..data.len()].copy_from_slice(data);

    cache.entries.insert(
        block,
        CacheEntry {
            data: full_data,
            dirty: true,
            access_count: counter,
        },
    );
    cache.total_bytes += bs;

    Ok(())
}

/// Sync all dirty blocks to disk.
pub fn sync() -> Result<usize, &'static str> {
    let mut guard = CACHE.lock();
    let cache = guard.as_mut().ok_or("Block cache not initialized")?;
    let write_fn = cache.write_fn.ok_or("No write function")?;

    let mut synced = 0;
    for (block, entry) in cache.entries.iter_mut() {
        if entry.dirty {
            write_fn(*block, &entry.data)?;
            entry.dirty = false;
            synced += 1;
        }
    }
    Ok(synced)
}

/// Get cache statistics.
pub fn stats() -> (usize, usize, usize) {
    let guard = CACHE.lock();
    match guard.as_ref() {
        Some(cache) => {
            let dirty = cache.entries.values().filter(|e| e.dirty).count();
            (cache.entries.len(), dirty, cache.total_bytes)
        }
        None => (0, 0, 0),
    }
}

fn evict_lru(cache: &mut CacheInner) {
    if cache.entries.is_empty() {
        return;
    }

    // Find the least recently used entry
    let lru_block = cache
        .entries
        .iter()
        .min_by_key(|(_, e)| e.access_count)
        .map(|(&k, _)| k);

    if let Some(block) = lru_block {
        if let Some(entry) = cache.entries.remove(&block) {
            // Write back if dirty
            if entry.dirty {
                if let Some(write_fn) = cache.write_fn {
                    let _ = write_fn(block, &entry.data);
                }
            }
            cache.total_bytes -= cache.block_size;
        }
    }
}
