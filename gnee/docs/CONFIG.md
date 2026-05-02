# GNEE Configuration Guide

## Kernel Configuration

### Required Kernel Options

```bash
# KVM/Virtualization
CONFIG_KVM=y
CONFIG_KVM_ARM64=y
CONFIG_KVM_ARM_PMU=y
CONFIG_VIRTUALIZATION=y

# IOMMU/SMMU
CONFIG_IOMMU_SUPPORT=y
CONFIG_ARM_SMMU=y
CONFIG_IOMMU_DMA=y

# DMA Buffer
CONFIG_DMA_SHARED_BUFFER=y
CONFIG_ION=y
CONFIG_ION_SYSTEM_HEAP=y

# DRM/GPU
CONFIG_DRM=y
CONFIG_DRM_KMS_HELPER=y
CONFIG_DRM_VIRTIO_GPU=y

# Performance
CONFIG_PREEMPT=y
CONFIG_HIGH_RES_TIMERS=y
```

### Recommended Boot Parameters

```bash
kvm-arm.mode=nvhe
cma=256M@0x80000000
hugepagesz=2M
hugepages=1024
iommu.passthrough=0
iommu.strict=0
isolcpus=1-3
nohz_full=1-3
rcu_nocbs=1-3
irqaffinity=0
gpu.max_buffer_size=128M
```

## Runtime Configuration

### config.toml

```toml
[hal]
rooted = false
smmu_passthrough = false
dma_buf_sharing = true

[jit]
enable_optimizations = true
enable_caching = true
cache_size_limit = 67108864

[graphics]
fsr_enabled = true
mfg_enabled = false
upscale_factor = 1.5

[thermal]
heartbeat_enabled = true
target_temp = 40
max_temp = 50
```

## Performance Tuning

### CPU Tuning
```bash
taskset -c 1-3 gnee_app
renice -n -20 -p $(pidof gnee_app)
```

### GPU Tuning
```bash
echo 800 | tee /sys/devices/platform/kgsl-3d0/devfreq/kgsl-3d0/max_freq
```

### Memory Tuning
```bash
echo 1024 | tee /proc/sys/vm/nr_hugepages
```

## License
SPDX-License-Identifier: GPL-2.0
