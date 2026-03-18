/// ChaCha20-based CSPRNG.
///
/// Generates a keystream block of 64 bytes using ChaCha20 quarter-round
/// operations, then serves random bytes from that block. Reseeds after
/// every 1 MiB of output.
pub struct ChaCha20Rng {
    state: [u32; 16],
    buffer: [u8; 64],
    buffer_pos: usize,
    bytes_since_reseed: u64,
}

const RESEED_INTERVAL: u64 = 1024 * 1024; // 1 MiB

impl ChaCha20Rng {
    /// Create a new ChaCha20 CSPRNG from a 32-byte seed.
    pub fn new(seed: &[u8; 32]) -> Self {
        let mut state = [0u32; 16];
        // "expand 32-byte k" constants
        state[0] = 0x6170_7865;
        state[1] = 0x3320_646E;
        state[2] = 0x7962_2D32;
        state[3] = 0x6B20_6574;

        // Key (8 words from seed)
        for i in 0..8 {
            state[4 + i] = u32::from_le_bytes([
                seed[i * 4],
                seed[i * 4 + 1],
                seed[i * 4 + 2],
                seed[i * 4 + 3],
            ]);
        }

        // Counter and nonce start at 0
        state[12] = 0;
        state[13] = 0;
        state[14] = 0;
        state[15] = 0;

        let mut rng = Self {
            state,
            buffer: [0; 64],
            buffer_pos: 64, // Force generation on first use
            bytes_since_reseed: 0,
        };
        rng.generate_block();
        rng
    }

    /// Fill a buffer with random bytes.
    pub fn fill(&mut self, buf: &mut [u8]) {
        let mut offset = 0;
        while offset < buf.len() {
            if self.buffer_pos >= 64 {
                self.generate_block();
            }
            let available = 64 - self.buffer_pos;
            let needed = buf.len() - offset;
            let to_copy = available.min(needed);
            buf[offset..offset + to_copy]
                .copy_from_slice(&self.buffer[self.buffer_pos..self.buffer_pos + to_copy]);
            self.buffer_pos += to_copy;
            offset += to_copy;
            self.bytes_since_reseed += to_copy as u64;

            if self.bytes_since_reseed >= RESEED_INTERVAL {
                self.reseed_from_rdrand();
            }
        }
    }

    fn generate_block(&mut self) {
        let mut working = self.state;

        // 20 rounds (10 double-rounds)
        for _ in 0..10 {
            // Column rounds
            quarter_round(&mut working, 0, 4, 8, 12);
            quarter_round(&mut working, 1, 5, 9, 13);
            quarter_round(&mut working, 2, 6, 10, 14);
            quarter_round(&mut working, 3, 7, 11, 15);
            // Diagonal rounds
            quarter_round(&mut working, 0, 5, 10, 15);
            quarter_round(&mut working, 1, 6, 11, 12);
            quarter_round(&mut working, 2, 7, 8, 13);
            quarter_round(&mut working, 3, 4, 9, 14);
        }

        // Add original state
        for (w, s) in working.iter_mut().zip(self.state.iter()) {
            *w = w.wrapping_add(*s);
        }

        // Serialize to bytes
        for (i, &word) in working.iter().enumerate() {
            let bytes = word.to_le_bytes();
            self.buffer[i * 4..i * 4 + 4].copy_from_slice(&bytes);
        }
        self.buffer_pos = 0;

        // Increment counter
        self.state[12] = self.state[12].wrapping_add(1);
        if self.state[12] == 0 {
            self.state[13] = self.state[13].wrapping_add(1);
        }
    }

    fn reseed_from_rdrand(&mut self) {
        let mut extra = [0u8; 32];
        let mut got_entropy = true;
        for chunk in extra.chunks_exact_mut(8) {
            let val: u64;
            let ok: u8;
            // SAFETY: RDRAND instruction, may fail on some hardware.
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
                got_entropy = false;
                break;
            }
            chunk.copy_from_slice(&val.to_le_bytes());
        }

        if got_entropy {
            // XOR new entropy into key portion of state
            for i in 0..8 {
                let new_word = u32::from_le_bytes([
                    extra[i * 4],
                    extra[i * 4 + 1],
                    extra[i * 4 + 2],
                    extra[i * 4 + 3],
                ]);
                self.state[4 + i] ^= new_word;
            }
        }

        self.bytes_since_reseed = 0;
    }
}

#[inline]
fn quarter_round(s: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
    s[a] = s[a].wrapping_add(s[b]);
    s[d] ^= s[a];
    s[d] = s[d].rotate_left(16);

    s[c] = s[c].wrapping_add(s[d]);
    s[b] ^= s[c];
    s[b] = s[b].rotate_left(12);

    s[a] = s[a].wrapping_add(s[b]);
    s[d] ^= s[a];
    s[d] = s[d].rotate_left(8);

    s[c] = s[c].wrapping_add(s[d]);
    s[b] ^= s[c];
    s[b] = s[b].rotate_left(7);
}
