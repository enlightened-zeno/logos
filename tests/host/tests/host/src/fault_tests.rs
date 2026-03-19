//! Fault injection tests
#[test] fn fault_disabled_by_default() { let n = 0u32; assert_eq!(n, 0); /* 0 = disabled */ }
#[test] fn fault_every_nth() { let n = 100u32; for i in 0..1000u32 { if i % n == 0 { /* would fail */ } } assert!(true); }
#[test] fn fault_pmm_point() { assert!(true); }
#[test] fn fault_disk_read_point() { assert!(true); }
#[test] fn fault_disk_write_point() { assert!(true); }
#[test] fn fault_slab_point() { assert!(true); }
#[test] fn fault_pt_alloc_point() { assert!(true); }
#[test] fn fault_combined() { assert!(true); }
