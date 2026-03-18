# Unsafe Code Audit Log

All `unsafe` blocks in the LogOS kernel have `// SAFETY:` comments.
This file is auto-generated. Run `./scripts/check-unsafe-audit.sh` to verify.

## Summary

| File | Unsafe Blocks |
|------|--------------|
| main.rs | 31 |
| fs/ext2/mod.rs | 17 |
| memory/slab.rs | 12 |
| memory/heap.rs | 11 |
| drivers/virtio/block.rs | 11 |
| sched/mod.rs | 6 |
| memory/paging.rs | 6 |
| drivers/ahci.rs | 6 |
| arch/x86_64/io.rs | 6 |
| arch/x86_64/gdt.rs | 6 |
| arch/x86_64/apic.rs | 6 |
| arch/x86_64/cpu.rs | 5 |
| timer/mod.rs | 4 |
| process/address_space.rs | 4 |
| memory/vmm.rs | 4 |
| memory/pmm.rs | 4 |
| drivers/framebuffer.rs | 4 |
| arch/x86_64/syscall.rs | 4 |
| arch/x86_64/idt.rs | 4 |
| syscall/validate.rs | 3 |
| sync/rwlock.rs | 3 |
| process/task.rs | 3 |
| sync/spinlock.rs | 2 |
| sync/mutex.rs | 2 |
| process/exec.rs | 2 |
| process/elf.rs | 2 |
| log.rs | 2 |
| entropy/mod.rs | 2 |
| drivers/keyboard.rs | 2 |
| arch/x86_64/smp.rs | 2 |
| shell/builtins.rs | 1 |
| entropy/chacha20.rs | 1 |

**Total: 178 unsafe blocks, all with `// SAFETY:` comments.**

## Categories

- **Inline assembly:** CPU instructions (HLT, CLI, STI, CPUID, MSR, I/O ports)
- **Raw pointers:** Page table manipulation, MMIO access, DMA buffers
- **Static mutable state:** Boot-time initialization of global singletons
- **FFI:** Naked functions for context switch, syscall entry
- **Memory operations:** Zero-fill, copy, volatile reads/writes
