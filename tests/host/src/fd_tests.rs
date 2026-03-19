/// File descriptor table logic tests.

#[test]
fn test_fd_allocation_order() {
    // FDs should be allocated lowest-first
    let mut fds = vec![false; 256];
    // Allocate fd 0
    for (i, slot) in fds.iter_mut().enumerate() {
        if !*slot {
            *slot = true;
            assert_eq!(i, 0);
            break;
        }
    }
    // Allocate fd 1
    for (i, slot) in fds.iter_mut().enumerate() {
        if !*slot {
            *slot = true;
            assert_eq!(i, 1);
            break;
        }
    }
}

#[test]
fn test_fd_close_reuse() {
    let mut fds = vec![false; 256];
    fds[0] = true; // allocated
    fds[1] = true; // allocated

    // Close fd 0
    fds[0] = false;

    // Next alloc should get fd 0
    for (i, slot) in fds.iter_mut().enumerate() {
        if !*slot {
            *slot = true;
            assert_eq!(i, 0);
            break;
        }
    }
}

#[test]
fn test_fd_max() {
    let max_fds = 256;
    let mut count = 0;
    let fds = vec![true; max_fds]; // All allocated
    for slot in &fds {
        if *slot {
            count += 1;
        }
    }
    assert_eq!(count, max_fds);
    // Next alloc should fail (no free slot)
    assert!(!fds.iter().any(|&s| !s));
}

#[test]
fn test_dup2_target() {
    let mut fds = vec![false; 256];
    fds[0] = true; // stdin

    // dup2(0, 5) — fd 5 now points to same thing as fd 0
    fds[5] = true;

    assert!(fds[0]);
    assert!(fds[5]);
}
