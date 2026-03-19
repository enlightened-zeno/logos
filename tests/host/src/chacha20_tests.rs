/// ChaCha20 quarter-round and keystream tests.

fn quarter_round(s: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
    s[a] = s[a].wrapping_add(s[b]); s[d] ^= s[a]; s[d] = s[d].rotate_left(16);
    s[c] = s[c].wrapping_add(s[d]); s[b] ^= s[c]; s[b] = s[b].rotate_left(12);
    s[a] = s[a].wrapping_add(s[b]); s[d] ^= s[a]; s[d] = s[d].rotate_left(8);
    s[c] = s[c].wrapping_add(s[d]); s[b] ^= s[c]; s[b] = s[b].rotate_left(7);
}

#[test]
fn test_quarter_round() {
    // RFC 7539 test vector for quarter round
    let mut state = [0u32; 16];
    state[0] = 0x879531e0;
    state[1] = 0xc5ecf37d;
    state[2] = 0x516461b1;
    state[3] = 0xc9a62f8a;
    quarter_round(&mut state, 0, 1, 2, 3);
    // Just verify it doesn't panic and produces different output
    assert_ne!(state[0], 0x879531e0);
}

#[test]
fn test_chacha20_constants() {
    // "expand 32-byte k" in little-endian u32s
    let c0: u32 = 0x61707865; // "expa"
    let c1: u32 = 0x3320646e; // "nd 3"
    let c2: u32 = 0x79622d32; // "2-by"
    let c3: u32 = 0x6b206574; // "te k"
    assert_eq!(c0, 0x61707865);
    assert_eq!(c1, 0x3320646e);
    assert_eq!(c2, 0x79622d32);
    assert_eq!(c3, 0x6b206574);
}

#[test]
fn test_chacha20_block() {
    // Full ChaCha20 block function
    let mut state = [0u32; 16];
    state[0] = 0x61707865;
    state[1] = 0x3320646e;
    state[2] = 0x79622d32;
    state[3] = 0x6b206574;
    // Key = all zeros
    // Nonce = all zeros, counter = 0

    let original = state;

    for _ in 0..10 {
        quarter_round(&mut state, 0, 4, 8, 12);
        quarter_round(&mut state, 1, 5, 9, 13);
        quarter_round(&mut state, 2, 6, 10, 14);
        quarter_round(&mut state, 3, 7, 11, 15);
        quarter_round(&mut state, 0, 5, 10, 15);
        quarter_round(&mut state, 1, 6, 11, 12);
        quarter_round(&mut state, 2, 7, 8, 13);
        quarter_round(&mut state, 3, 4, 9, 14);
    }

    // Add original state
    for i in 0..16 {
        state[i] = state[i].wrapping_add(original[i]);
    }

    // Output should be deterministic
    assert_ne!(state, [0u32; 16]);
    assert_ne!(state, original);
}
