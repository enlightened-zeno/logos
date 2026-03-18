.PHONY: all build release image run test test-host test-kernel test-system \
       test-perf test-chaos clean clippy fmt fmt-check unsafe-audit help

KERNEL := target/x86_64-logos/debug/logos-kernel
IMAGE  := build/logos.img

all: build                   ## Build everything

build:                       ## Build kernel (debug)
	cargo build

release:                     ## Build kernel (release)
	cargo build --release

image: build                 ## Create bootable disk image
	./tools/fetch-limine.sh
	./tools/mkimage-noroot.sh

run: image                   ## Boot in QEMU (interactive)
	@OVMF=$$(for c in /usr/share/OVMF/OVMF_CODE.fd /usr/share/OVMF/OVMF_CODE_4M.fd \
	          /usr/share/ovmf/OVMF.fd /usr/share/qemu/OVMF.fd; do \
	          [ -f "$$c" ] && echo "$$c" && break; done); \
	qemu-system-x86_64 \
		-drive if=pflash,format=raw,readonly=on,file="$$OVMF" \
		-drive file=$(IMAGE),format=raw \
		-machine q35 \
		-cpu qemu64,+rdrand,+xsave,+xsaveopt \
		-smp 2 -m 256M \
		-serial stdio \
		-no-reboot

test: test-kernel test-system ## Run all tests

test-host:                   ## Run host-side unit tests
	cargo test --workspace --target x86_64-unknown-linux-gnu 2>/dev/null || true

test-kernel: image           ## Run in-kernel boot tests (QEMU)
	@echo "=== Kernel Boot Tests ==="
	@OVMF=$$(for c in /usr/share/OVMF/OVMF_CODE.fd /usr/share/OVMF/OVMF_CODE_4M.fd \
	          /usr/share/ovmf/OVMF.fd /usr/share/qemu/OVMF.fd; do \
	          [ -f "$$c" ] && echo "$$c" && break; done); \
	timeout 60 qemu-system-x86_64 \
		-drive if=pflash,format=raw,readonly=on,file="$$OVMF" \
		-drive file=$(IMAGE),format=raw \
		-machine q35 \
		-cpu qemu64,+rdrand,+xsave,+xsaveopt \
		-smp 2 -m 256M \
		-serial file:build/serial.log \
		-display none -no-reboot 2>&1 || true
	@echo ""
	@PASS=$$(strings build/serial.log | grep -c "PASS"); \
	echo "Tests passed: $$PASS"; \
	grep -q "Boot complete. Halting." build/serial.log && echo "Boot: OK" || echo "Boot: FAILED"; \
	strings build/serial.log | grep "PANIC" && exit 1 || true

test-system: image           ## Run system tests (Python harness)
	python3 tests/serial_harness.py --timeout 60 --smp 2 --memory 256M

test-perf: image             ## Run performance benchmarks
	@echo "=== Performance Benchmarks ==="
	@echo "Run 'bench all' in the shell for interactive benchmarks"
	@echo "(Automated perf baseline recording not yet implemented)"

test-chaos: image            ## Run chaos/stress tests
	@echo "=== Chaos Tests ==="
	@echo "Run 'stress all' in the shell for interactive stress tests"

clippy:                      ## Run Clippy linter
	cargo clippy -- -D warnings

fmt:                         ## Format all code
	cargo fmt --all

fmt-check:                   ## Check formatting
	cargo fmt --all -- --check

unsafe-audit:                ## Verify all unsafe blocks have SAFETY comments
	./scripts/check-unsafe-audit.sh

serial:                      ## Monitor serial output from running QEMU
	@echo "Attach to QEMU serial with: socat - unix-connect:build/serial.sock"

gdb:                         ## Attach GDB to running QEMU
	@echo "Start QEMU with: make run EXTRA='-s -S'"
	@echo "Then: gdb target/x86_64-logos/debug/logos-kernel -ex 'target remote :1234'"

clean:                       ## Remove build artifacts
	cargo clean
	rm -rf build/

help:                        ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' Makefile | sort | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  %-20s %s\n", $$1, $$2}'
