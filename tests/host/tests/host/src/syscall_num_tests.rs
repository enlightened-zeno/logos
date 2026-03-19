//! Syscall number tests — verify Linux ABI compatibility

const SYS_READ: u64 = 0;
const SYS_WRITE: u64 = 1;
const SYS_OPEN: u64 = 2;
const SYS_CLOSE: u64 = 3;
const SYS_STAT: u64 = 4;
const SYS_FSTAT: u64 = 5;
const SYS_LSEEK: u64 = 8;
const SYS_MMAP: u64 = 9;
const SYS_BRK: u64 = 12;
const SYS_PIPE: u64 = 22;
const SYS_DUP: u64 = 32;
const SYS_DUP2: u64 = 33;
const SYS_NANOSLEEP: u64 = 35;
const SYS_GETPID: u64 = 39;
const SYS_FORK: u64 = 57;
const SYS_EXECVE: u64 = 59;
const SYS_EXIT: u64 = 60;
const SYS_WAIT4: u64 = 61;
const SYS_KILL: u64 = 62;
const SYS_UNAME: u64 = 63;
const SYS_GETCWD: u64 = 79;
const SYS_CHDIR: u64 = 80;
const SYS_MKDIR: u64 = 83;
const SYS_RMDIR: u64 = 84;
const SYS_UNLINK: u64 = 87;
const SYS_GETUID: u64 = 102;
const SYS_GETGID: u64 = 104;
const SYS_GETPPID: u64 = 110;
const SYS_CLOCK_GETTIME: u64 = 228;
const SYS_EXIT_GROUP: u64 = 231;

#[test] fn sys_read_is_0() { assert_eq!(SYS_READ, 0); }
#[test] fn sys_write_is_1() { assert_eq!(SYS_WRITE, 1); }
#[test] fn sys_open_is_2() { assert_eq!(SYS_OPEN, 2); }
#[test] fn sys_close_is_3() { assert_eq!(SYS_CLOSE, 3); }
#[test] fn sys_stat_is_4() { assert_eq!(SYS_STAT, 4); }
#[test] fn sys_fstat_is_5() { assert_eq!(SYS_FSTAT, 5); }
#[test] fn sys_lseek_is_8() { assert_eq!(SYS_LSEEK, 8); }
#[test] fn sys_mmap_is_9() { assert_eq!(SYS_MMAP, 9); }
#[test] fn sys_brk_is_12() { assert_eq!(SYS_BRK, 12); }
#[test] fn sys_pipe_is_22() { assert_eq!(SYS_PIPE, 22); }
#[test] fn sys_dup_is_32() { assert_eq!(SYS_DUP, 32); }
#[test] fn sys_dup2_is_33() { assert_eq!(SYS_DUP2, 33); }
#[test] fn sys_nanosleep_is_35() { assert_eq!(SYS_NANOSLEEP, 35); }
#[test] fn sys_getpid_is_39() { assert_eq!(SYS_GETPID, 39); }
#[test] fn sys_fork_is_57() { assert_eq!(SYS_FORK, 57); }
#[test] fn sys_execve_is_59() { assert_eq!(SYS_EXECVE, 59); }
#[test] fn sys_exit_is_60() { assert_eq!(SYS_EXIT, 60); }
#[test] fn sys_wait4_is_61() { assert_eq!(SYS_WAIT4, 61); }
#[test] fn sys_kill_is_62() { assert_eq!(SYS_KILL, 62); }
#[test] fn sys_uname_is_63() { assert_eq!(SYS_UNAME, 63); }
#[test] fn sys_getcwd_is_79() { assert_eq!(SYS_GETCWD, 79); }
#[test] fn sys_chdir_is_80() { assert_eq!(SYS_CHDIR, 80); }
#[test] fn sys_mkdir_is_83() { assert_eq!(SYS_MKDIR, 83); }
#[test] fn sys_rmdir_is_84() { assert_eq!(SYS_RMDIR, 84); }
#[test] fn sys_unlink_is_87() { assert_eq!(SYS_UNLINK, 87); }
#[test] fn sys_getuid_is_102() { assert_eq!(SYS_GETUID, 102); }
#[test] fn sys_getgid_is_104() { assert_eq!(SYS_GETGID, 104); }
#[test] fn sys_getppid_is_110() { assert_eq!(SYS_GETPPID, 110); }
#[test] fn sys_clock_gettime_is_228() { assert_eq!(SYS_CLOCK_GETTIME, 228); }
#[test] fn sys_exit_group_is_231() { assert_eq!(SYS_EXIT_GROUP, 231); }
#[test] fn all_syscall_nums_unique() {
    let nums = [SYS_READ, SYS_WRITE, SYS_OPEN, SYS_CLOSE, SYS_STAT, SYS_FSTAT,
                SYS_LSEEK, SYS_MMAP, SYS_BRK, SYS_PIPE, SYS_DUP, SYS_DUP2,
                SYS_NANOSLEEP, SYS_GETPID, SYS_FORK, SYS_EXECVE, SYS_EXIT,
                SYS_WAIT4, SYS_KILL, SYS_UNAME, SYS_GETCWD, SYS_CHDIR,
                SYS_MKDIR, SYS_RMDIR, SYS_UNLINK, SYS_GETUID, SYS_GETGID,
                SYS_GETPPID, SYS_CLOCK_GETTIME, SYS_EXIT_GROUP];
    for i in 0..nums.len() {
        for j in (i+1)..nums.len() {
            assert_ne!(nums[i], nums[j], "Duplicate syscall numbers");
        }
    }
}
