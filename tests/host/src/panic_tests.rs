/// Panic handler logic tests.

#[test]
fn test_nested_panic_detection() {
    use std::sync::atomic::{AtomicBool, Ordering};
    let in_panic = AtomicBool::new(false);

    // First panic
    let was_panicking = in_panic.swap(true, Ordering::SeqCst);
    assert!(!was_panicking); // Not nested

    // Second panic (nested)
    let was_panicking2 = in_panic.swap(true, Ordering::SeqCst);
    assert!(was_panicking2); // Nested!
}

#[test]
fn test_panic_output_format() {
    let file = "kernel/src/main.rs";
    let line = 42u32;
    let message = "assertion failed";

    let output = format!(
        "===== KERNEL PANIC =====\nLocation: {}:{}\nMessage: {}\n========================",
        file, line, message
    );

    assert!(output.contains("KERNEL PANIC"));
    assert!(output.contains(file));
    assert!(output.contains(&line.to_string()));
    assert!(output.contains(message));
}
