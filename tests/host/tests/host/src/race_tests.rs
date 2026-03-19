//! Race condition tests (host-side logic verification)
use std::sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}};
use std::thread;

#[test] fn counter_no_race() {
    let counter = Arc::new(AtomicU64::new(0));
    let mut handles = vec![];
    for _ in 0..4 {
        let c = counter.clone();
        handles.push(thread::spawn(move || { for _ in 0..10000 { c.fetch_add(1, Ordering::SeqCst); } }));
    }
    for h in handles { h.join().unwrap(); }
    assert_eq!(counter.load(Ordering::SeqCst), 40000);
}
#[test] fn mutex_protects_data() {
    let data = Arc::new(Mutex::new(Vec::new()));
    let mut handles = vec![];
    for i in 0..4u32 {
        let d = data.clone();
        handles.push(thread::spawn(move || { for _ in 0..100 { d.lock().unwrap().push(i); } }));
    }
    for h in handles { h.join().unwrap(); }
    assert_eq!(data.lock().unwrap().len(), 400);
}
#[test] fn concurrent_vec_push() {
    let v = Arc::new(Mutex::new(Vec::new()));
    let mut handles = vec![];
    for _ in 0..8 {
        let v = v.clone();
        handles.push(thread::spawn(move || { for i in 0..100 { v.lock().unwrap().push(i); } }));
    }
    for h in handles { h.join().unwrap(); }
    assert_eq!(v.lock().unwrap().len(), 800);
}
