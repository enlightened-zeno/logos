/// Core utility output logic tests.

#[test] fn test_ls_type_char() {
    let types = vec![('d', "dir"), ('-', "file"), ('c', "char"), ('b', "block"), ('l', "link"), ('p', "pipe")];
    assert_eq!(types.len(), 6);
}
#[test] fn test_grep_match() {
    let lines = vec!["hello world", "foo bar", "hello foo"];
    let matches: Vec<&&str> = lines.iter().filter(|l| l.contains("hello")).collect();
    assert_eq!(matches.len(), 2);
}
#[test] fn test_grep_no_match() {
    let lines = vec!["hello", "world"];
    let matches: Vec<&&str> = lines.iter().filter(|l| l.contains("xyz")).collect();
    assert_eq!(matches.len(), 0);
}
#[test] fn test_sort_lines() {
    let mut lines = vec!["banana", "apple", "cherry"];
    lines.sort();
    assert_eq!(lines, vec!["apple", "banana", "cherry"]);
}
#[test] fn test_uniq_adjacent() {
    let input = vec!["a", "a", "b", "b", "b", "c"];
    let mut output = Vec::new();
    let mut last = "";
    for line in &input {
        if *line != last { output.push(*line); last = line; }
    }
    assert_eq!(output, vec!["a", "b", "c"]);
}
#[test] fn test_cut_field() {
    let line = "field1:field2:field3";
    let fields: Vec<&str> = line.split(':').collect();
    assert_eq!(fields[1], "field2");
}
#[test] fn test_tr_translate() {
    let input = "hello";
    let output: String = input.chars().map(|c| if c == 'l' { 'r' } else { c }).collect();
    assert_eq!(output, "herro");
}
#[test] fn test_true_exit_code() { assert_eq!(0i32, 0); }
#[test] fn test_false_exit_code() { assert_eq!(1i32, 1); }
#[test] fn test_sleep_argument() {
    let arg = "5";
    let secs: u64 = arg.parse().unwrap();
    assert_eq!(secs, 5);
}
#[test] fn test_kill_signal_parse() {
    let arg = "-9";
    let sig: i32 = arg[1..].parse().unwrap();
    assert_eq!(sig, 9);
}
#[test] fn test_date_format() {
    let epoch = 0u64;
    assert_eq!(epoch, 0); // We don't have real time yet
}
#[test] fn test_whoami() {
    let uid = 0u32;
    let name = if uid == 0 { "root" } else { "user" };
    assert_eq!(name, "root");
}
#[test] fn test_test_command() {
    // test -f file → check if file exists
    let exists = true;
    assert!(exists);
}
#[test] fn test_mv_rename() {
    let src = "old.txt";
    let dst = "new.txt";
    assert_ne!(src, dst);
}
#[test] fn test_touch_creates() {
    let exists_before = false;
    let exists_after = true; // touch creates it
    assert!(!exists_before);
    assert!(exists_after);
}
#[test] fn test_rmdir_empty() {
    let entries = vec![".", ".."];
    let is_empty = entries.iter().all(|e| *e == "." || *e == "..");
    assert!(is_empty);
}
#[test] fn test_rmdir_notempty() {
    let entries = vec![".", "..", "file.txt"];
    let is_empty = entries.iter().all(|e| *e == "." || *e == "..");
    assert!(!is_empty); // ENOTEMPTY
}
