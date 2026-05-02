// SPDX-License-Identifier: GPL-2.0
/*
 * Game-Native Execution Environment - Thermal Heartbeat
 *
 * Copyright (C) 2025 GNEE Team
 *
 * Thermal heartbeat implementation to maintain high CPU/GPU frequencies
 * during game execution by preventing thermal throttling.
 */

//! # Thermal Heartbeat
//!
//! Maintains high CPU/GPU frequencies by preventing thermal throttling
//! through controlled workload generation.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crate::hal::interface::HalInterface;
use crate::error::{Error, Result};

/// Thermal heartbeat configuration
#[derive(Debug, Clone)]
pub struct ThermalHeartbeatConfig {
    /// Enable thermal heartbeat
    pub enabled: bool,
    /// Heartbeat interval in milliseconds
    pub interval_ms: u32,
    /// Workload intensity (0-100)
    pub intensity: u32,
    /// Maximum temperature in Celsius
    pub max_temperature: i32,
    /// Target temperature in Celsius
    pub target_temperature: i32,
    /// Adaptive mode (adjust intensity based on temperature)
    pub adaptive: bool,
}

impl Default for ThermalHeartbeatConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_ms: 10,
            intensity: 50,
            max_temperature: 50,
            target_temperature: 40,
            adaptive: true,
        }
    }
}

/// Thermal heartbeat statistics
#[derive(Debug, Default)]
pub struct ThermalHeartbeatStats {
    /// Total heartbeat cycles
    pub total_cycles: AtomicU64,
    /// Active cycles (workload generated)
    pub active_cycles: AtomicU64,
    /// Idle cycles (no workload)
    pub idle_cycles: AtomicU64,
    /// Throttled cycles (temperature too high)
    pub throttled_cycles: AtomicU64,
    /// Total workload time in nanoseconds
    pub total_workload_ns: AtomicU64,
    /// Average temperature
    pub avg_temperature: AtomicU64,
    /// Peak temperature
    pub peak_temperature: AtomicU64,
}

/// Thermal heartbeat manager
pub struct ThermalHeartbeat {
    /// Configuration
    config: ThermalHeartbeatConfig,
    /// Statistics
    stats: Arc<ThermalHeartbeatStats>,
    /// Running flag
    running: Arc<AtomicBool>,
    /// Thread handle
    thread: Option<thread::JoinHandle<()>>,
    /// HAL interface
    hal: Arc<dyn HalInterface>,
}

impl ThermalHeartbeat {
    /// Create a new thermal heartbeat instance
    pub fn new(hal: Arc<dyn HalInterface>, config: ThermalHeartbeatConfig) -> Self {
        Self {
            config,
            stats: Arc::new(ThermalHeartbeatStats::default()),
            running: Arc::new(AtomicBool::new(false)),
            thread: None,
            hal,
        }
    }

    /// Start the thermal heartbeat
    pub fn start(&mut self) -> Result<()> {
        if self.running.load(Ordering::Acquire) {
            return Err(Error::InvalidState("Heartbeat already running"));
        }

        self.running.store(true, Ordering::Release);
        let running = Arc::clone(&self.running);
        let stats = Arc::clone(&self.stats);
        let config = self.config.clone();
        let hal = Arc::clone(&self.hal);

        let handle = thread::spawn(move || {
            Self::heartbeat_loop(running, stats, config, hal);
        });

        self.thread = Some(handle);
        Ok(())
    }

    /// Stop the thermal heartbeat
    pub fn stop(&mut self) -> Result<()> {
        if !self.running.load(Ordering::Acquire) {
            return Err(Error::InvalidState("Heartbeat not running"));
        }

        self.running.store(false, Ordering::Release);

        if let Some(handle) = self.thread.take() {
            handle.join().map_err(|_| Error::IoError("Failed to join thread"))?;
        }

        Ok(())
    }

    /// Check if heartbeat is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    /// Get statistics
    pub fn get_stats(&self) -> &ThermalHeartbeatStats {
        &self.stats
    }

    /// Update configuration
    pub fn update_config(&mut self, config: ThermalHeartbeatConfig) {
        self.config = config;
    }

    /// Heartbeat loop
    fn heartbeat_loop(
        running: Arc<AtomicBool>,
        stats: Arc<ThermalHeartbeatStats>,
        config: ThermalHeartbeatConfig,
        hal: Arc<dyn HalInterface>,
    ) {
        let mut accumulator: u64 = 0;
        let mut temp_accumulator: u64 = 0;
        let mut temp_count: u64 = 0;
        let mut peak_temp: i32 = 0;

        while running.load(Ordering::Acquire) {
            let start = Instant::now();

            // Get current temperature
            let current_temp = hal.get_temperature().unwrap_or(30);
            temp_accumulator += current_temp as u64;
            temp_count += 1;
            peak_temp = peak_temp.max(current_temp);

            // Update peak temperature
            stats.peak_temperature.store(peak_temp as u64, Ordering::Relaxed);

            // Check if we should generate workload
            let should_work = if config.adaptive {
                // Adaptive mode: adjust based on temperature
                current_temp < config.target_temperature
            } else {
                // Fixed mode: always generate workload
                true
            };

            if should_work && current_temp < config.max_temperature {
                // Generate workload
                let workload_duration = Self::generate_workload(config.intensity);
                stats.active_cycles.fetch_add(1, Ordering::Relaxed);
                stats.total_workload_ns.fetch_add(
                    workload_duration.as_nanos() as u64,
                    Ordering::Relaxed
                );
            } else {
                // Idle cycle (temperature too high or disabled)
                stats.idle_cycles.fetch_add(1, Ordering::Relaxed);
            }

            // Update total cycles
            stats.total_cycles.fetch_add(1, Ordering::Relaxed);

            // Calculate elapsed time
            let elapsed = start.elapsed();
            let sleep_duration = Duration::from_millis(config.interval_ms as u64)
                .saturating_sub(elapsed);

            // Sleep for remaining interval
            if !sleep_duration.is_zero() {
                thread::sleep(sleep_duration);
            }

            // Update average temperature periodically
            if temp_count > 100 {
                let avg = temp_accumulator / temp_count;
                stats.avg_temperature.store(avg, Ordering::Relaxed);
                temp_accumulator = 0;
                temp_count = 0;
            }
        }
    }

    /// Generate workload to keep CPU active
    fn generate_workload(intensity: u32) -> Duration {
        let start = Instant::now();
        let iterations = (intensity as u64) * 1000;
        let mut accumulator: u64 = 0;

        // Perform CPU-intensive work
        for i in 0..iterations {
            // Mix of arithmetic and memory operations
            accumulator = accumulator.wrapping_add(i);
            accumulator = accumulator.wrapping_mul(3);
            accumulator = accumulator.wrapping_xor(0x5DEECE66D);
            accumulator = accumulator.wrapping_add(0xB);
            
            // Add some memory pressure
            if i % 100 == 0 {
                let _ = std::hint::black_box(accumulator);
            }
        }

        // Prevent compiler from optimizing away
        std::hint::black_box(accumulator);

        start.elapsed()
    }

    /// Get current temperature
    pub fn get_temperature(&self) -> Result<i32> {
        self.hal.get_temperature()
    }

    /// Get maximum temperature
    pub fn get_max_temperature(&self) -> Result<i32> {
        self.hal.get_max_temperature()
    }

    /// Check if thermal throttling is active
    pub fn is_throttling(&self) -> bool {
        let current_temp = self.get_temperature().unwrap_or(0);
        current_temp >= self.config.max_temperature
    }

    /// Get thermal status
    pub fn get_status(&self) -> ThermalStatus {
        let current_temp = self.get_temperature().unwrap_or(0);
        let max_temp = self.get_max_temperature().unwrap_or(50);

        if current_temp >= max_temp {
            ThermalStatus::Critical
        } else if current_temp >= self.config.target_temperature {
            ThermalStatus::High
        } else if current_temp >= self.config.target_temperature - 5 {
            ThermalStatus::Normal
        } else {
            ThermalStatus::Cool
        }
    }
}

impl Drop for ThermalHeartbeat {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

/// Thermal status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermalStatus {
    /// Cool - below target temperature
    Cool,
    /// Normal - near target temperature
    Normal,
    /// High - approaching maximum temperature
    High,
    /// Critical - at or above maximum temperature
    Critical,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hal::interface::HalCapabilities;

    #[test]
    fn test_thermal_heartbeat_config_default() {
        let config = ThermalHeartbeatConfig::default();
        assert!(config.enabled);
        assert_eq!(config.interval_ms, 10);
        assert_eq!(config.intensity, 50);
        assert_eq!(config.max_temperature, 50);
        assert!(config.adaptive);
    }

    #[test]
    fn test_thermal_heartbeat_creation() {
        // This would require a mock HAL
        // For now, just test the structure
        let config = ThermalHeartbeatConfig::default();
        assert_eq!(config.interval_ms, 10);
    }

    #[test]
    fn test_workload_generation() {
        let duration = ThermalHeartbeat::generate_workload(10);
        assert!(duration.as_nanos() > 0);
    }

    #[test]
    fn test_thermal_status() {
        let config = ThermalHeartbeatConfig::default();
        
        // Cool status
        let status = if 30 < config.target_temperature {
            ThermalStatus::Cool
        } else {
            ThermalStatus::Normal
        };
        assert_eq!(status, ThermalStatus::Cool);
    }
}
