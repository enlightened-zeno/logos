#!/bin/bash
# Run in-kernel tests in QEMU. Used as cargo test runner.
# QEMU exit codes: 33 = tests passed, 35 = tests failed, 124 = timeout

KERNEL_BINARY="$1"
TEST_NAME="$(basename "$KERNEL_BINARY")"
SERIAL_LOG="build/test-output/${TEST_NAME}.log"

mkdir -p build/test-output

QEMU_SMP="${QEMU_SMP:-2}"
QEMU_MEM="${QEMU_MEM:-256M}"

timeout 60 qemu-system-x86_64 \
    -machine q35 \
    -cpu qemu64,+rdrand,+xsave,+xsaveopt \
    -smp "$QEMU_SMP" \
    -m "$QEMU_MEM" \
    -serial file:"${SERIAL_LOG}" \
    -display none \
    -device isa-debug-exit,iobase=0xf4,iosize=0x04 \
    -no-reboot \
    -kernel "${KERNEL_BINARY}" \
    2>/dev/null

EXIT_CODE=$?

# ISA debug exit device: QEMU exit code = (written_value << 1) | 1
# We write 0x10 for success => exit 33
# We write 0x11 for failure => exit 35
# timeout exit code = 124

if [ $EXIT_CODE -eq 33 ]; then
    cat "${SERIAL_LOG}"
    exit 0
elif [ $EXIT_CODE -eq 124 ]; then
    echo "ERROR: Test timed out after 60 seconds"
    cat "${SERIAL_LOG}" 2>/dev/null
    exit 1
else
    echo "ERROR: Tests failed (QEMU exit code: $EXIT_CODE)"
    cat "${SERIAL_LOG}" 2>/dev/null
    exit 1
fi
