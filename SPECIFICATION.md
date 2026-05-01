# Game-Native Execution Environment (GNEE)
## Architectural Specification & Implementation Guide

**Version:** 1.0.0  
**Target:** Android (ARM64)  
**Status:** Active Development  

---

## 1. Core Architecture: Polymorphic HAL

The engine utilizes a `HalInterface` to abstract hardware access. Logic remains unified (JIT/Shim/API); hardware interaction switches based on privileges.

### 1.1 Privilege Levels

#### Mode: Rooted (EL2/Hypervisor)
*   **CPU:** Direct register mapping; bypassing Android kernel scheduler.
*   **Memory:** Kernel-level page table manipulation; Pinned Physical RAM.
*   **GPU:** Direct SMMU passthrough; Command stream register injection.
*   **Interrupts:** Direct GIC (Generic Interrupt Controller) handling.

#### Mode: Non-Rooted (User-Space)
*   **CPU:** `sched_affinity` (pinning to "Gold" cores); Thermal Heartbeats (busy-wait threads) for DVFS locking.
*   **Memory:** `mlockall` + `mmap` with two-stage `mprotect` (W^X compliance).
*   **GPU:** User-Space Vulkan batch optimization; Descriptor set batching.
*   **Interrupts:** Polling via `perf_event_open` or `ioctl` timers.

### 1.2 HAL Interface Definition

```rust
/// Hardware Abstraction Layer Interface
/// Determines execution path based on detected privileges
pub trait HalInterface {
    /// Memory Management
    fn allocate_pinned_memory(size: usize, flags: MemFlags) -> Result<PhysicalRegion>;
    fn map_physical(addr: PhysAddr, size: usize) -> Result<VirtualAddr>;
    
    /// CPU Control
    fn pin_cpu(core_id: u32) -> Result<()>;
    fn set_priority(priority: ThreadPriority) -> Result<()>;
    
    /// GPU Control
    fn submit_command_buffer(cmd: &CommandBuffer) -> Result<Fence>;
    fn wait_fence(fence: Fence, timeout_ms: u32) -> Result<()>;
    
    /// Interrupts
    fn register_interrupt(irq: u32, handler: InterruptHandler) -> Result<()>;
}
```

---

## 2. Operational Modes

### 2.1 Game Mode (High Performance)

*   **Aggressive Hoarding:**
    *   RAM pinning via `mlock` (prevents swapping).
    *   CPU pinning to performance cores (Cortex-X/A78).
    *   Thermal Heartbeat active (maintains high clock speeds).
*   **Graphics:**
    *   MFG (Multi-Frame Generation) active.
    *   FSR Sidecar Compute Kernels running.
    *   Async Compute enabled.
*   **Execution:**
    *   JIT Engine running in high-performance state.
    *   Aggressive inlining and loop unrolling.

### 2.2 System/Background Mode (Efficiency)

*   **Release:**
    *   Normal resource priority.
    *   Thermal Heartbeat disabled (saves battery).
*   **Graphics:**
    *   Fallback to standard Vulkan presentation.
    *   No MFG/FSR upscaling.
*   **Execution:**
    *   Standard JIT optimization levels.
    *   Power-saving thread scheduling.

---

## 3. Graphics Pipeline (The "Sidecar" Approach)

### 3.1 Strategy
Do not rely on Guest OS graphics services. Intercept and offload.

### 3.2 Implementation Flow

1.  **Intercept:** API Shim hooks `Present()` / `SwapBuffers()`.
2.  **Offload:** Generation tasks sent to "Sidecar" Compute Kernel.
3.  **Execute:** GPU partition processes upscaling/frame generation.
4.  **Present:** Zero-Copy buffer shared to display compositor.

### 3.3 Zero-Copy Sharing

*   **Rooted:** SMMU mapping allows direct physical address sharing.
*   **Non-Rooted:** `ASHMEM` + `ION` allocator with `dma_buf` file descriptors.

---

## 4. Execution Engine

### 4.1 JIT Compiler Pipeline

```
x86-64 Binary 
    ↓
[Decoder] → Decode to Intermediate Representation (IR)
    ↓
[Optimizer] → Constant Folding, Dead Code Elimination, Register Allocation
    ↓
[ARM64 Emitter] → Emit native ARM64 instructions
    ↓
[Memory Ordering] → Inject DMB/ISB for TSO emulation
    ↓
Native ARM64 Code
```

### 4.2 Memory Ordering (TSO Emulation)

x86 uses Total Store Order (TSO); ARM uses Weak Memory Ordering.
*   **Load-Load:** No barrier needed (ARM guarantees).
*   **Store-Store:** No barrier needed (ARM guarantees).
*   **Load-Store:** No barrier needed.
*   **Store-Load:** **Requires `DMB LD`** (Data Memory Barrier Load).
*   **Atomic Operations:** Require `LDAXR`/`STLXR` (Load-Acquire/Store-Release).

### 4.3 API Shim

*   Direct syscall mapping (Windows API → Native ARM64 Linux syscalls).
*   No Box64/Winlator overhead layer.
*   Custom `ntdll` implementation for NT kernel emulation.

---

## 5. Deployment Model

### 5.1 Frontend
*   **Platform:** Android APK
*   **UI Framework:** Jetpack Compose / Flutter
*   **Role:** Control plane, settings, game library management.

### 5.2 Backend

#### Rooted Devices
*   **Mechanism:** Magisk / KernelSU module.
*   **Loading:** Loads hypervisor at boot or via `kexec`.
*   **Privileges:** Full EL2 access.

#### Non-Rooted Devices
*   **Mechanism:** Foreground Service.
*   **Priority:** `FOREGROUND_SERVICE_TYPE_NONE` with high priority.
*   **Limitations:** No EL2 access, relies on user-space optimizations.

---

## 6. Directory Structure

```
gnee/
├── hal/                   # Hardware Abstraction Layer
│   ├── rooted/            # EL2/Hypervisor implementations
│   ├── user/              # User-space implementations
│   └── interface.rs       # HAL trait definition
├── jit/                   # JIT Compiler
│   ├── decoder/           # x86-64 decoder
│   ├── ir/                # Intermediate Representation
│   ├── optimizer/         # Optimization passes
│   └── emitter/           # ARM64 code emitter
├── graphics/              # Graphics Sidecar
│   ├── vulkan/            # Vulkan backend
│   ├── mfg/               # Multi-Frame Generation
│   └── fsr/               # FidelityFX Super Resolution
├── shim/                  # API Shim
│   ├── windows/           # Windows API emulation
│   └── syscall/           # Syscall translation
├── memory/                # Memory Management
│   ├── ion/               # ION allocator
│   └── smmu/              # SMMU management
├── thermal/               # Thermal Management
│   └── heartbeat.rs       # Thermal heartbeat logic
└── frontend/              # Android UI
    ├── app/               # Main application
    └── compose/           # UI components
```

---

## 7. Implementation Roadmap

### Phase 1: Foundation
- [ ] Define HAL interface traits
- [ ] Implement basic user-space HAL
- [ ] Create JIT decoder skeleton
- [ ] Setup build system (Cargo + Gradle)

### Phase 2: Graphics
- [ ] Implement Vulkan backend
- [ ] Create Sidecar compute pipeline
- [ ] Add FSR upscaling kernel
- [ ] Implement Zero-Copy buffer sharing

### Phase 3: Hypervisor (Rooted)
- [ ] Write EL2 entry points (ARM64 ASM)
- [ ] Implement Stage-2 MMU
- [ ] Add SMMU passthrough
- [ ] Create Magisk module installer

### Phase 4: Optimization
- [ ] Add thermal heartbeat
- [ ] Implement CPU pinning strategies
- [ ] Optimize JIT memory ordering
- [ ] Add MFG frame generation

### Phase 5: Frontend
- [ ] Build Android UI
- [ ] Add game library management
- [ ] Create settings interface
- [ ] Implement foreground service

---

## 8. Security Considerations

*   **W^X Compliance:** Memory pages must be either Writeable or Executable, never both.
*   **Sandboxing:** JIT code runs in isolated memory regions.
*   **Validation:** All input binaries validated before JIT compilation.
*   **Permissions:** Minimal permission set for non-rooted mode.

---

## 9. Performance Targets

| Metric | Target (Rooted) | Target (Non-Rooted) |
|--------|-----------------|---------------------|
| JIT Overhead | < 5% | < 10% |
| Graphics Overhead | < 2% | < 5% |
| Memory Pinning | 100% | ~80% (via mlock) |
| CPU Pinning | Direct | Via affinity |
| Frame Time Variance | < 1ms | < 3ms |

---

## 10. References

*   ARMv8-A Architecture Reference Manual
*   Vulkan Specification 1.3
*   Linux Kernel Documentation (ION, DMA-BUF, SMMU)
*   Android NDK Documentation
*   x86-64 System V ABI
