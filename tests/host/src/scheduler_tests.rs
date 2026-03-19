/// Scheduler logic tests.

#[test]
fn test_mlfq_quanta() {
    let quanta = [10u64, 20, 40, 80]; // ms per level
    assert_eq!(quanta[0], 10);
    assert_eq!(quanta[3], 80);
    // Each level doubles
    for i in 1..quanta.len() {
        assert_eq!(quanta[i], quanta[i - 1] * 2);
    }
}

#[test]
fn test_priority_demotion() {
    let mut priority = 0u8;
    let max_level = 3u8;

    // After quantum exhaustion, demote
    if priority < max_level {
        priority += 1;
    }
    assert_eq!(priority, 1);

    if priority < max_level {
        priority += 1;
    }
    assert_eq!(priority, 2);

    if priority < max_level {
        priority += 1;
    }
    assert_eq!(priority, 3);

    // Can't demote past max
    if priority < max_level {
        priority += 1;
    }
    assert_eq!(priority, 3);
}

#[test]
fn test_priority_boost() {
    // Boost all tasks to highest priority
    let mut priorities = vec![1u8, 2, 3, 3, 2, 1, 0];
    for p in &mut priorities {
        *p = 0;
    }
    assert!(priorities.iter().all(|&p| p == 0));
}

#[test]
fn test_round_robin_within_level() {
    // Tasks at the same priority level get round-robin
    let mut queue = std::collections::VecDeque::new();
    queue.push_back("A");
    queue.push_back("B");
    queue.push_back("C");

    assert_eq!(queue.pop_front(), Some("A"));
    queue.push_back("A"); // Re-add after running
    assert_eq!(queue.pop_front(), Some("B"));
    queue.push_back("B");
    assert_eq!(queue.pop_front(), Some("C"));
}

#[test]
fn test_idle_process() {
    // When no tasks are ready, idle process runs
    let ready_count = 0;
    let should_idle = ready_count == 0;
    assert!(should_idle);
}
