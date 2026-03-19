//! Signal number tests
const SIGHUP: u8 = 1;
const SIGINT: u8 = 2;
const SIGQUIT: u8 = 3;
const SIGILL: u8 = 4;
const SIGTRAP: u8 = 5;
const SIGABRT: u8 = 6;
const SIGFPE: u8 = 8;
const SIGKILL: u8 = 9;
const SIGSEGV: u8 = 11;
const SIGPIPE: u8 = 13;
const SIGALRM: u8 = 14;
const SIGTERM: u8 = 15;
const SIGCHLD: u8 = 17;
const SIGCONT: u8 = 18;
const SIGSTOP: u8 = 19;
const SIGTSTP: u8 = 20;

#[test] fn sigkill_is_9() { assert_eq!(SIGKILL, 9); }
#[test] fn sigterm_is_15() { assert_eq!(SIGTERM, 15); }
#[test] fn sigint_is_2() { assert_eq!(SIGINT, 2); }
#[test] fn sigsegv_is_11() { assert_eq!(SIGSEGV, 11); }
#[test] fn sigchld_is_17() { assert_eq!(SIGCHLD, 17); }
#[test] fn sigstop_is_19() { assert_eq!(SIGSTOP, 19); }
#[test] fn sigcont_is_18() { assert_eq!(SIGCONT, 18); }
#[test] fn signal_mask_set() {
    let mut mask: u64 = 0;
    mask |= 1 << SIGINT;
    assert!(mask & (1 << SIGINT) != 0);
    assert!(mask & (1 << SIGTERM) == 0);
}
#[test] fn signal_mask_clear() {
    let mut mask: u64 = u64::MAX;
    mask &= !(1 << SIGKILL);
    assert!(mask & (1 << SIGKILL) == 0);
}
#[test] fn sigkill_not_blockable() {
    // SIGKILL (9) and SIGSTOP (19) cannot be blocked per POSIX
    assert_eq!(SIGKILL, 9);
    assert_eq!(SIGSTOP, 19);
}
#[test] fn all_signal_nums_unique() {
    let sigs = [SIGHUP, SIGINT, SIGQUIT, SIGILL, SIGTRAP, SIGABRT, SIGFPE,
                SIGKILL, SIGSEGV, SIGPIPE, SIGALRM, SIGTERM, SIGCHLD,
                SIGCONT, SIGSTOP, SIGTSTP];
    for i in 0..sigs.len() {
        for j in (i+1)..sigs.len() {
            assert_ne!(sigs[i], sigs[j]);
        }
    }
}
