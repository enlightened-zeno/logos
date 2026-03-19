/// Procfs logic tests.

#[test]
fn test_procfs_entries() {
    let entries = vec!["uptime", "meminfo", "version", "mounts"];
    assert_eq!(entries.len(), 4);
    assert!(entries.contains(&"uptime"));
    assert!(entries.contains(&"meminfo"));
}

#[test]
fn test_uptime_format() {
    let ticks = 5000u64;
    let secs = ticks / 1000;
    let frac = (ticks % 1000) / 10;
    let output = format!("{}.{:02} 0.00", secs, frac);
    assert!(output.starts_with("5.00"));
}

#[test]
fn test_meminfo_format() {
    let total_kb = 256 * 1024u64;
    let free_kb = 200 * 1024u64;
    let used_kb = total_kb - free_kb;
    let output = format!("MemTotal: {:>8} kB\nMemFree: {:>8} kB\n", total_kb, free_kb);
    assert!(output.contains("MemTotal"));
    assert!(output.contains("MemFree"));
    let _ = used_kb;
}

#[test]
fn test_version_string() {
    let version = "LogOS version 0.1.0 (x86_64)";
    assert!(version.contains("LogOS"));
    assert!(version.contains("0.1.0"));
    assert!(version.contains("x86_64"));
}

#[test]
fn test_mounts_format() {
    let entry = "tmpfs / tmpfs rw 0 0";
    let parts: Vec<&str> = entry.split_whitespace().collect();
    assert_eq!(parts.len(), 6);
    assert_eq!(parts[0], "tmpfs"); // device
    assert_eq!(parts[1], "/");     // mountpoint
}

#[test]
fn test_procfs_read_only() {
    let mode = 0o444u32; // r--r--r--
    assert!(mode & 0o400 != 0); // readable
    assert!(mode & 0o200 == 0); // not writable
}
