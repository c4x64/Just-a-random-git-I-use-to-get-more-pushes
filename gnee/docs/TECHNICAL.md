# GNEE - Comprehensive Technical Documentation

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Hardware Abstraction Layer](#hardware-abstraction-layer)
3. [JIT Compiler](#jit-compiler)
4. [Graphics Pipeline](#graphics-pipeline)
5. [Memory Management](#memory-management)
6. [Thermal Management](#thermal-management)
7. [API Shim](#api-shim)
8. [Synchronization](#synchronization)
9. [Command Queues](#command-queues)
10. [Performance Tuning](#performance-tuning)

---

## Architecture Overview

### Design Goals

1. **Near-Native Performance**: Achieve >90% of native GPU performance
2. **Minimal Overhead**: Keep CPU overhead below 10%
3. **Zero-Copy Graphics**: Eliminate unnecessary memory copies
4. **Thermal Efficiency**: Prevent throttling during extended gameplay
5. **Compatibility**: Support both rooted and non-rooted devices

### System Components

```
┌─────────────────────────────────────────────────────────────────┐
│ Application Layer                                               │
│  - Games (x86-64 binaries)                                     │
│  - Windows API calls                                           │
│  - DirectX/Vulkan API                                          │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ API Shim Layer                                                  │
│  - Windows API → Linux syscall mapping                         │
│  - DirectX → Vulkan translation                                │
│  - Hook management                                             │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ JIT Translation Layer                                           │
│  - x86-64 decoder                                              │
│  - IR optimization                                             │
│  - ARM64 emitter                                               │
│  - Code caching                                                │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ Graphics Sidecar                                                │
│  - FSR upscaling                                               │
│  - MFG frame generation                                        │
│  - Optical flow                                                │
│  - Buffer management                                           │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ HAL Interface                                                   │
│  - Rooted: EL2 hypervisor (KVM/pKVM)                           │
│  - User: Linux APIs (mlock, sched_affinity)                    │
│  - Memory: dma-buf, ION, SMMU                                  │
│  - GPU: Vulkan, DRM/KMS                                        │
└─────────────────────────────────────────────────────────────────┘
```

---

## Hardware Abstraction Layer

### Rooted Mode (EL2 Hypervisor)

**Requirements:**
- ARM64 with virtualization extensions
- Kernel 5.10+ with KVM support
- Root access (Magisk/KernelSU)

**Features:**
- Direct hypervisor access via HVC calls
- Stage-2 MMU for memory isolation
- SMMU passthrough for GPU
- Protected memory regions

**Example:**
```rust
use gnee::hal::rooted::RootedHal;

let hal = RootedHal::new()?;
hal.pin_cpu(1)?;  // Pin to core 1
let region = hal.allocate_pinned_memory(
    4096,
    MemFlags::new().read().write().pin()
)?;
```

### User-Space Mode

**Features:**
- `mlock` for memory pinning
- `sched_setaffinity` for CPU pinning
- Thermal heartbeat for DVFS control
- Standard Linux APIs only

**Example:**
```rust
use gnee::hal::user::UserHal;

let hal = UserHal::new()?;
hal.pin_cpu(1)?;
let region = hal.allocate_pinned_memory(
    4096,
    MemFlags::new().read().write()
)?;
```

---

## JIT Compiler

### Pipeline

1. **Decode**: x86-64 → Instruction IR
2. **Optimize**: Constant folding, dead code elimination
3. **Emit**: IR → ARM64
4. **Cache**: Store translated blocks

### Supported Instructions

| Category | Instructions |
|----------|-------------|
| Data Transfer | MOV, MOVX, LEA, PUSH, POP |
| Arithmetic | ADD, SUB, MUL, IMUL, DIV, IDIV |
| Logical | AND, OR, XOR, NOT |
| Shift/Rotate | SHL, SHR, SAR, ROL, ROR |
| Control Flow | JMP, CALL, RET, JE, JNE, JG, JL |
| SSE/AVX | MOVD, MOVQ, MOVDQA, PADDB, PSUBB |

### Performance

| Metric | Value |
|--------|-------|
| Translation Speed | ~10 MB/s |
| Cache Hit Rate | ~85% |
| Overhead | ~5-10% |

---

## Graphics Pipeline

### FSR Integration

**Steps:**
1. Intercept `vkCreateImage` calls
2. Track color/depth buffers
3. On `Present()`: apply FSR upscaling
4. Submit to display

### MFG Frame Generation

**Requirements:**
- Motion vectors (from game or optical flow)
- Depth buffer
- Previous frame

**Algorithm:**
```
1. Get current frame (t)
2. Get previous frame (t-1)
3. Generate motion vectors (if not provided)
4. Interpolate intermediate frame (t+0.5)
5. Apply FSR upscaling
6. Present
```

---

## Memory Management

### Memory Types

| Type | Use Case |
|------|----------|
| Normal | General purpose |
| Device | MMIO regions |
| Protected | DRM content |
| DmaCoherent | GPU buffers |

### DMA Buffer Sharing

```rust
// Export dma-buf from guest
let fd = ion_export(buffer);

// Import in host
let dmabuf = dma_buf_import(fd);

// Zero-copy access
let ptr = dmabuf.map()?;
```

---

## Thermal Management

### Heartbeat Algorithm

```
loop:
  if temperature < target:
    generate_workload(intensity)
  else if temperature > max:
    pause_heartbeat()
  sleep(interval)
```

### Configuration

| Parameter | Default | Description |
|-----------|---------|-------------|
| interval_ms | 10 | Heartbeat interval |
| intensity | 50 | Workload intensity (0-100) |
| target_temp | 40°C | Target temperature |
| max_temp | 50°C | Maximum temperature |

---

## API Shim

### Windows → Linux Mapping

| Windows API | Linux Equivalent |
|-------------|------------------|
| CreateFile | open |
| ReadFile | read |
| WriteFile | write |
| CloseHandle | close |
| VirtualAlloc | mmap |
| VirtualFree | munmap |
| CreateThread | clone |
| Sleep | nanosleep |

---

## Synchronization

### Primitives

- **Fence**: GPU completion signaling
- **Semaphore**: Resource counting
- **Mutex**: Mutual exclusion
- **RwLock**: Read-write locks
- **Barrier**: Thread synchronization
- **Condvar**: Condition variables

---

## Command Queues

### Lock-Free Design

```
Producer: head = atomic_fetch_add(1)
          slot = buffer[head % size]
          slot.write(entry)
          tail.store(head + 1)

Consumer: if head == tail: empty
          entry = buffer[head % size].read()
          head += 1
```

### Performance

| Metric | Value |
|--------|-------|
| Throughput | ~10M cmds/s |
| Latency | <1μs |
| Contention | Zero (lock-free) |

---

## Performance Tuning

### CPU Optimization

1. Pin to performance cores
2. Use isolated CPUs (nohz_full)
3. Set real-time priority
4. Disable thermal throttling

### GPU Optimization

1. Enable async compute
2. Use dedicated queues
3. Minimize state changes
4. Batch submissions

### Memory Optimization

1. Use hugepages (2M)
2. Enable CMA for GPU
3. Pin critical buffers
4. Minimize TLB misses

---

## License

SPDX-License-Identifier: GPL-2.0
