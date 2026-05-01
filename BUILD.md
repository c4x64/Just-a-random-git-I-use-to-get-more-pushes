# Game-Native Execution Environment - Build Guide

## Prerequisites

### System Requirements
- **OS:** Linux (Ubuntu 20.04+ recommended)
- **Architecture:** ARM64 (aarch64) or x86_64 for cross-compilation
- **Kernel:** 5.10+ with KVM support
- **Rust:** 1.70+ (for user-space components)
- **NDK:** r25+ (for Android integration)
- **Gradle:** 8.0+ (for Android frontend)

### Android Device Requirements
- **CPU:** ARMv8-A with Cortex-A76 or newer
- **GPU:** Adreno 600+ or Mali-G76+
- **RAM:** 8GB minimum (12GB recommended)
- **Storage:** 2GB free space minimum

## Building

### 1. User-Space HAL (Non-Rooted)

```bash
# Clone repository
git clone https://github.com/gnee-project/gnee.git
cd gnee

# Build user-space HAL
cargo build --release -p gnee-hal-user

# Build all user-space components
cargo build --release --features "user"
```

### 2. Rooted HAL (Hypervisor)

**Warning:** Requires kernel module compilation and root access.

```bash
# Set kernel headers path
export KERNEL_DIR=/path/to/kernel/headers

# Build kernel module
make -C kernel/ CROSS_COMPILE=aarch64-linux-gnu- ARCH=arm64

# Install kernel module
sudo insmod gnee_hal_rooted.ko

# Verify installation
lsmod | grep gnee
```

### 3. Android Frontend

```bash
# Navigate to frontend
cd frontend/

# Build debug APK
./gradlew assembleDebug

# Build release APK (requires signing)
./gradlew assembleRelease

# Install on device
adb install app/build/outputs/apk/debug/app-debug.apk
```

### 4. Full Build (All Components)

```bash
# Build everything
./build.sh all

# Build with specific features
./build.sh --features rooted,thermal,fsr

# Clean build
./build.sh clean
```

## Configuration

### Kernel Module Parameters

```bash
# Load with debug logging
sudo modprobe gnee_hal_rooted debug=1

# Load with thermal heartbeat disabled
sudo modprobe gnee_hal_rooted thermal_heartbeat=0

# Set CPU pinning policy
sudo modprobe gnee_hal_rooted cpu_policy=performance
```

### Runtime Configuration

Create `/etc/gnee/config.toml`:

```toml
[hal]
mode = "auto"  # auto, rooted, or user
debug = false

[graphics]
mfg_enabled = true
fsr_enabled = true
upscale_factor = 1.5

[thermal]
heartbeat_enabled = true
max_temperature = 45

[memory]
pinned_size_mb = 2048
use_ion = true
```

## Testing

### Unit Tests

```bash
# Run all tests
cargo test

# Run with coverage
cargo tarpaulin --out Html

# Run specific test
cargo test --package gnee-hal --lib hal::tests
```

### Integration Tests

```bash
# Run integration tests
./tests/run_integration.sh

# Run performance benchmarks
./tests/benchmark.sh

# Run compatibility tests
./tests/compat.sh
```

## Deployment

### Rooted Devices (Magisk)

```bash
# Build Magisk module
./scripts/build_magisk.sh

# Install via adb
adb push gnee-magisk.zip /data/local/tmp/
adb shell su -c "magisk --install /data/local/tmp/gnee-magisk.zip"
adb reboot
```

### Non-Rooted Devices

```bash
# Install APK
adb install gnee-app.apk

# Grant permissions
adb shell pm grant com.gnee.engine android.permission.FOREGROUND_SERVICE

# Start service
adb shell am start-foreground-service com.gnee.engine/.GneeService
```

## Troubleshooting

### Common Issues

#### "KVM not supported"
- Ensure kernel has `CONFIG_KVM=y`
- Check `/dev/kvm` exists
- Verify CPU virtualization extensions enabled

#### "ION allocation failed"
- Check ION heap availability
- Verify `CONFIG_ION=y` in kernel
- Try reducing pinned memory size

#### "Thermal heartbeat not working"
- Ensure thermal daemon is running
- Check `/sys/class/thermal/` entries
- Verify HAL has thermal permissions

### Debug Logging

```bash
# Enable kernel debug logging
echo "module gnee_hal_rooted +p" | sudo tee /sys/kernel/debug/dynamic_debug/control

# View logs
dmesg | grep GNEE
logcat | grep GNEE
```

### Performance Profiling

```bash
# Profile with perf
perf record -g -p $(pidof gnee)
perf report

# Trace GPU submissions
sudo trace-cmd record -e drm:drm_vblank_event
sudo trace-cmd report
```

## Performance Tuning

### Optimal Settings

| Setting | Value | Description |
|---------|-------|-------------|
| `cpu_policy` | performance | Maximum CPU frequency |
| `gpu_policy` | performance | Maximum GPU frequency |
| `thermal_threshold` | 50 | Start throttling at 50°C |
| `memory_pin_size` | 2048 | Pin 2GB RAM |
| `mfg_factor` | 2.0 | Double frame generation |

### Battery Optimization

For battery-conscious usage:

```toml
[thermal]
heartbeat_enabled = false
max_temperature = 40

[graphics]
mfg_enabled = false
fsr_enabled = false

[memory]
pinned_size_mb = 512
```

## License

GPL-2.0

## Support

- **Documentation:** https://gnee.dev/docs
- **Issues:** https://github.com/gnee-project/gnee/issues
- **Discord:** https://discord.gg/gnee
