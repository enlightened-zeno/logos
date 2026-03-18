#!/usr/bin/env python3
"""
LogOS system test harness.

Boots LogOS in QEMU, captures serial output, and verifies
test assertions. Used by CI and `make test-system`.

Usage:
    python3 tests/serial_harness.py [--timeout 60] [--smp 2] [--memory 256M]
"""

import subprocess
import sys
import os
import argparse
import re
import time

def find_ovmf():
    """Find OVMF firmware on the system."""
    candidates = [
        "/usr/share/OVMF/OVMF_CODE.fd",
        "/usr/share/OVMF/OVMF_CODE_4M.fd",
        "/usr/share/ovmf/OVMF.fd",
        "/usr/share/qemu/OVMF.fd",
    ]
    for path in candidates:
        if os.path.exists(path):
            return path
    return None

def run_qemu(image, ovmf, smp, memory, timeout):
    """Boot QEMU and capture serial output."""
    serial_log = "build/serial-test.log"

    cmd = [
        "qemu-system-x86_64",
        "-drive", f"if=pflash,format=raw,readonly=on,file={ovmf}",
        "-drive", f"file={image},format=raw",
        "-machine", "q35",
        "-cpu", "qemu64,+rdrand,+xsave,+xsaveopt",
        "-smp", str(smp),
        "-m", memory,
        "-serial", f"file:{serial_log}",
        "-display", "none",
        "-no-reboot",
    ]

    try:
        subprocess.run(cmd, timeout=timeout, capture_output=True)
    except subprocess.TimeoutExpired:
        # Expected — kernel halts, QEMU doesn't exit
        subprocess.run(["pkill", "-f", "qemu-system-x86_64"], capture_output=True)
        time.sleep(1)

    if not os.path.exists(serial_log):
        return ""

    # Read and clean serial output
    with open(serial_log, "rb") as f:
        raw = f.read()

    # Extract printable strings
    text = ""
    for line in raw.split(b"\n"):
        try:
            decoded = line.decode("utf-8", errors="ignore")
            # Strip ANSI escape codes and control chars
            cleaned = re.sub(r'\x1b\[[0-9;]*[a-zA-Z]', '', decoded)
            cleaned = re.sub(r'[\x00-\x08\x0b\x0c\x0e-\x1f]', '', cleaned)
            if cleaned.strip():
                text += cleaned + "\n"
        except Exception:
            continue

    return text

def check_tests(output):
    """Parse test results from serial output."""
    passed = []
    failed = []
    panics = []

    for line in output.split("\n"):
        line = line.strip()
        if "TEST" in line and "PASS" in line:
            # Extract test name
            match = re.search(r'TEST (.+?):\s*PASS', line)
            if match:
                passed.append(match.group(1))
        if "KERNEL PANIC" in line:
            panics.append(line)

    return passed, failed, panics

def main():
    parser = argparse.ArgumentParser(description="LogOS system test harness")
    parser.add_argument("--timeout", type=int, default=60, help="QEMU timeout in seconds")
    parser.add_argument("--smp", type=int, default=2, help="Number of CPUs")
    parser.add_argument("--memory", default="256M", help="RAM size")
    parser.add_argument("--image", default="build/logos.img", help="Disk image path")
    args = parser.parse_args()

    ovmf = find_ovmf()
    if not ovmf:
        print("ERROR: OVMF firmware not found")
        sys.exit(1)

    if not os.path.exists(args.image):
        print(f"ERROR: Disk image not found: {args.image}")
        print("Run 'cargo build && ./tools/mkimage-noroot.sh' first")
        sys.exit(1)

    print(f"LogOS System Test Harness")
    print(f"  OVMF:    {ovmf}")
    print(f"  Image:   {args.image}")
    print(f"  SMP:     {args.smp}")
    print(f"  Memory:  {args.memory}")
    print(f"  Timeout: {args.timeout}s")
    print()

    output = run_qemu(args.image, ovmf, args.smp, args.memory, args.timeout)

    if not output:
        print("ERROR: No serial output captured")
        sys.exit(1)

    passed, failed, panics = check_tests(output)

    # Required assertions
    required = [
        ("Boot complete", "Boot complete" in output),
        ("Shell prompt", "logos#" in output),
        ("No panics", len(panics) == 0),
        ("All memory tests", "All memory tests passed" in output),
        ("All VFS tests", "All VFS tests passed" in output),
        ("All extended tests", "All extended tests passed" in output),
    ]

    print(f"=== Test Results ===")
    print(f"Tests passed: {len(passed)}")
    for name in passed:
        print(f"  [PASS] {name}")

    if panics:
        print(f"\nPANICS:")
        for p in panics:
            print(f"  {p}")

    print(f"\n=== Required Checks ===")
    all_ok = True
    for name, ok in required:
        status = "OK" if ok else "FAIL"
        print(f"  [{status}] {name}")
        if not ok:
            all_ok = False

    print(f"\n{'PASSED' if all_ok else 'FAILED'}: {len(passed)} tests, {len(panics)} panics")

    sys.exit(0 if all_ok else 1)

if __name__ == "__main__":
    main()
