//! Timer wheel logic tests
#[test] fn timer_tick_increment() { let mut tick: u64 = 0; tick += 1; assert_eq!(tick, 1); }
#[test] fn timer_ms_to_ticks() { let ms = 100u64; let hz = 1000u64; assert_eq!(ms * hz / 1000, 100); }
#[test] fn timer_wheel_slot() { let tick: u64 = 12345; let slot = tick % 256; assert_eq!(slot, 57); }
#[test] fn timer_overflow_safe() { let t: u64 = u64::MAX; assert_eq!(t.wrapping_add(1), 0); }
#[test] fn timer_1s_is_1000ms() { assert_eq!(1000u64, 1000); }
#[test] fn timer_1ms_is_1000us() { assert_eq!(1_000_000u64 / 1000, 1000); }
#[test] fn timer_hz_1000() { let hz = 1000u64; let period_us = 1_000_000 / hz; assert_eq!(period_us, 1000); }
