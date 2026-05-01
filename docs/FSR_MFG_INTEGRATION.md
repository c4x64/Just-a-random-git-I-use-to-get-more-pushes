# Module Update: Graphics Pipeline (FSR/MFG Integration)

**Status:** Planned (Post-Initial Release)  
**Priority:** High  
**Target:** Android ARM64 (Mali-G68 / Adreno)  

---

## 1. The Integration Strategy

### 1.1 Direct SDK Linking
Integrate the AMD FidelityFX SDK as a static library within the `libgame_engine.so`.

*   **Action:** Compile FidelityFX CS (Compute Shader) sources to SPIR-V/IR.
*   **Linking:** Statically link the FSR dispatcher into the Sidecar Compute Module.
*   **Optimization:** Strip unused FSR presets; keep only performance-critical paths (Performance/Ultra-Performance).

### 1.2 Buffer Proxying
The API Shim must intercept rendering calls to monitor resource creation.

*   **Intercept Points:**
    *   **Vulkan:** `vkCmdDrawIndexed`, `vkCmdDrawIndirect`, `vkCreateImage`.
    *   **DirectX (via Shim):** `ID3D12GraphicsCommandList::DrawIndexedInstanced`.
*   **Tracking:**
    *   Identify Color Buffers (`VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT`).
    *   Identify Depth Buffers (`VK_IMAGE_USAGE_DEPTH_STENCIL_ATTACHMENT_BIT`).
    *   Identify Velocity Buffers (if available).
*   **Mechanism:**
    *   Wrap `VkImage` / `ID3D12Resource` handles.
    *   Maintain a shadow registry of active frames and their associated buffers.

### 1.3 Texture Re-binding
When the FSR/MFG SDK requires access, the runtime must provide direct pointers.

*   **Zero-Copy Access:**
    *   Use `vkGetImageMemoryRequirements` + `vkBindImageMemory` with imported DMA-buf.
    *   Avoid `vkMapMemory` host copies; use GPU-local memory.
*   **SDK Handoff:**
    *   Pass `VkImageView` or raw GPU handles directly to the FidelityFX context.
    *   Ensure layout transitions (`VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL`) are synchronized via semaphores.

---

## 2. The Frame Generation (MFG) Hurdle

### 2.1 Requirement: Motion Vectors
MFG (Multi-Frame Generation) strictly requires per-pixel motion vectors to reconstruct intermediate frames.

### 2.2 The Problem
*   **Modern Games:** Often render motion vectors to a separate buffer but discard them after post-processing (motion blur).
*   **Older Titles:** Do not compute motion vectors at all.
*   **API Limitation:** No standard API to "request" motion vectors from the guest application.

### 2.3 The Solution: The "Sidecar" Optical Flow Engine
If the game does not provide usable motion vectors, the Sidecar Engine must synthesize them.

#### Implementation Plan:
1.  **Detection:**
    *   Shim checks for `VK_IMAGE_USAGE_SAMPLED_BIT` + `VK_FORMAT_R16G16_SFLOAT` (common motion vector format).
    *   If found, hook the bind point and forward to MFG.
2.  **Synthesis (Optical Flow):**
    *   If no motion vectors are detected, trigger the **Optical Flow Compute Shader**.
    *   **Inputs:**
        *   Current Frame (Color + Depth).
        *   Previous Frame (Color + Depth).
        *   Camera Motion Matrix (if interceptable from uniform buffers).
    *   **Algorithm:**
        *   Lightweight Lucas-Kanade or Horn-Schunck variant optimized for mobile GPU.
        *   Utilize Depth Buffer for occlusion handling.
    *   **Output:** Synthetic Motion Vector Buffer (`R16G16_SFLOAT`).

---

## 3. Hardware Constraints (Exynos 1380 / Mali-G68)

### 3.1 Memory Alignment
FSR and MFG passes are sensitive to memory layout for optimal tile buffer usage on Mali.

*   **Alignment Requirement:** Ensure `VkImageCreateInfo::size` is aligned to 64KB (Mali tile size) or system page size (4KB/16KB).
*   **Usage Flags:**
    ```c
    VkImageCreateInfo imageInfo = {
        .usage = VK_IMAGE_USAGE_SAMPLED_BIT | 
                 VK_IMAGE_USAGE_STORAGE_BIT |  // Required for FSR compute
                 VK_IMAGE_USAGE_TRANSFER_DST_BIT,
        .tiling = VK_IMAGE_TILING_OPTIMAL,      // Mandatory for Mali
        .flags = VK_IMAGE_CREATE_MUTABLE_FORMAT_BIT
    };
    ```
*   **Validation:** Use `VK_ANDROID_external_memory_android_hardware_buffer` to ensure compatibility with Android's Gralloc/Hardware Composer.

### 3.2 Compute Queue Utilization
Maximize parallelism to hide latency.

*   **Async Compute:**
    *   Submit FSR/MFG passes to the **Async Compute Queue**.
    *   **Goal:** Allow the Main Queue to process Frame N+1 geometry while the Compute Queue upscales Frame N.
*   **Mali-Specific Optimization:**
    *   Mali GPUs use a Unified Memory Architecture (UMA).
    *   Avoid explicit cache flushes (`vkFlushMappedMemoryRanges`) by using `VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT`.
    *   Ensure `VkSubmitInfo::pWaitSemaphores` correctly orders Main Queue -> Compute Queue handoff.

---

## 4. Implementation Roadmap

### Phase 1: Buffer Interception (Weeks 1-3)
- [ ] Implement `VkImage` wrapper in API Shim.
- [ ] Add logic to detect Color/Depth/Velocity buffers.
- [ ] Create registry for active frame resources.

### Phase 2: FSR Integration (Weeks 4-6)
- [ ] Compile FidelityFX CS for Mali-G68 (OpenGL ES / Vulkan).
- [ ] Implement zero-copy buffer handoff.
- [ ] Test upscaling quality/performance on Exynos 1380.

### Phase 3: Optical Flow Engine (Weeks 7-10)
- [ ] Write lightweight Optical Flow compute shader.
- [ ] Integrate depth-based occlusion handling.
- [ ] Tune for mobile GPU performance (target < 2ms frame time).

### Phase 4: MFG & Async Compute (Weeks 11-14)
- [ ] Implement frame generation logic using synthetic vectors.
- [ ] Configure Async Compute queue submission.
- [ ] Final latency and power consumption tuning.

---

## 5. Code Structure Additions

```
gnee/
├── graphics/
│   ├── sidecar/
│   │   ├── fsr/
│   │   │   ├── fsr_context.rs       # FSR state management
│   │   │   ├── fsr_pass.rs          # Compute pass implementation
│   │   │   └── shaders/             # SPIR-V binaries
│   │   ├── mfg/
│   │   │   ├── mfg_context.rs       # MFG state management
│   │   │   ├── optical_flow.rs      # Synthetic vector generation
│   │   │   └── shaders/
│   │   └── interceptor/
│   │       ├── vk_interceptor.rs    # Vulkan API hooks
│   │       └── resource_tracker.rs  # Buffer tracking
```

---

## 6. Performance Targets (Exynos 1380)

| Metric | Target | Notes |
|--------|--------|-------|
| FSR Overhead | < 1.5ms | 1080p -> 1440p |
| Optical Flow | < 2.0ms | Synthetic vectors |
| MFG Total | < 4.0ms | Including synthesis |
| Memory Bandwidth | < 10 GB/s | Critical for battery |
| Frame Latency | < 10ms | End-to-end |

---

## 7. References
*   AMD FidelityFX SDK Documentation
*   Mali-G68 Developer Guide
*   Vulkan Async Compute Best Practices
*   Optical Flow on Mobile GPUs (Research Paper)
