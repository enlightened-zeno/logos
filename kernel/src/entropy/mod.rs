pub mod chacha20;

use crate::sync::SpinLock;

static RNG: SpinLock<Option<chacha20::ChaCha20Rng>> = SpinLock::new(None);

/// Initialize the CSPRNG with entropy from RDRAND or TSC jitter.
///
/// # Safety
/// Must be called once during boot.
pub unsafe fn init() {
    let mut seed = [0u8; 32];
    if !seed_from_rdrand(&mut seed) {
        seed_from_tsc_jitter(&mut seed);
    }
    *RNG.lock() = Some(chacha20::ChaCha20Rng::new(&seed));
    crate::serial_println!("Entropy: CSPRNG seeded");
}

/// Fill a buffer with cryptographically secure random bytes.
pub fn fill_bytes(buf: &mut [u8]) {
    let mut guard = RNG.lock();
    let rng = guard.as_mut().expect("CSPRNG not initialized");
    rng.fill(buf);
}

/// Get a random u64.
pub fn random_u64() -> u64 {
    let mut buf = [0u8; 8];
    fill_bytes(&mut buf);
    u64::from_le_bytes(buf)
}

/// Try to seed from RDRAND (preferred, hardware RNG).
fn seed_from_rdrand(seed: &mut [u8; 32]) -> bool {
    for chunk in seed.chunks_exact_mut(8) {
        let val: u64;
        let ok: u8;
        // SAFETY: RDRAND is checked for availability before calling.
        unsafe {
            core::arch::asm!(
                "rdrand {val}",
                "setc {ok}",
                val = out(reg) val,
                ok = out(reg_byte) ok,
                options(nomem, nostack)
            );
        }
        if ok == 0 {
            return false;
        }
        chunk.copy_from_slice(&val.to_le_bytes());
    }
    true
}

/// Fallback: seed from TSC jitter (lower quality but always available).
fn seed_from_tsc_jitter(seed: &mut [u8; 32]) {
    for byte in seed.iter_mut() {
        let mut accum: u8 = 0;
        for bit in 0..8 {
            let t1 = rdtsc();
            // Busy loop to introduce jitter
            for _ in 0..100 {
                core::hint::spin_loop();
            }
            let t2 = rdtsc();
            accum |= ((t2.wrapping_sub(t1) & 1) as u8) << bit;
        }
        *byte = accum;
    }
}

fn rdtsc() -> u64 {
    let (lo, hi): (u32, u32);
    // SAFETY: RDTSC is always available on x86_64.
    unsafe {
        core::arch::asm!("rdtsc", out("eax") lo, out("edx") hi, options(nomem, nostack));
    }
    (hi as u64) << 32 | lo as u64
}
