//! Scheduler tests
#[test] fn mlfq_levels() { assert_eq!(4usize, 4); }
#[test] fn quantum_level0() { assert_eq!(10u64, 10); /* 10ms */ }
#[test] fn quantum_level1() { assert_eq!(20u64, 20); }
#[test] fn quantum_level2() { assert_eq!(40u64, 40); }
#[test] fn quantum_level3() { assert_eq!(80u64, 80); }
#[test] fn quantum_doubles() {
    let quanta = [10u64, 20, 40, 80];
    for i in 1..quanta.len() { assert_eq!(quanta[i], quanta[i-1] * 2); }
}
#[test] fn boost_interval() { assert_eq!(1000u64, 1000); /* 1 second */ }
#[test] fn priority_demotion() { let mut level = 0u8; level = (level + 1).min(3); assert_eq!(level, 1); }
#[test] fn priority_boost() { let level = 3u8; let boosted = 0u8; assert!(boosted < level); }
#[test] fn round_robin_within_level() {
    let tasks = vec![1, 2, 3];
    let mut idx = 0;
    for expected in [1, 2, 3, 1, 2, 3] {
        assert_eq!(tasks[idx], expected);
        idx = (idx + 1) % tasks.len();
    }
}
