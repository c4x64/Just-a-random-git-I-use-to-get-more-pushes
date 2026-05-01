/* SPDX-License-Identifier: GPL-2.0 */
/*
 * Game-Native Execution Environment - Rooted HAL Implementation
 *
 * Copyright (C) 2025 GNEE Team
 *
 * EL2/Hypervisor implementation of the HAL interface.
 * Provides direct hardware access with minimal overhead.
 */

#ifndef _GNEE_HAL_ROOTED_H_
#define _GNEE_HAL_ROOTED_H_

#include <linux/types.h>
#include <linux/device.h>
#include <linux/dma-buf.h>
#include <linux/ion.h>
#include <linux/kvm.h>
#include <linux/kvm_arm.h>
#include <asm/cputype.h>
#include <asm/pgtable.h>

/* Rooted HAL flags */
#define GNEE_HAL_ROOTED_FLAG_HYPERVISOR    (1 << 0)
#define GNEE_HAL_ROOTED_FLAG_SMMU          (1 << 1)
#define GNEE_HAL_ROOTED_FLAG_ION           (1 << 2)
#define GNEE_HAL_ROOTED_FLAG_THERMAL       (1 << 3)

/* Hypervisor HVC call numbers */
#define GNEE_HVC_MEMORY_ALLOC    0x82000000
#define GNEE_HVC_MEMORY_FREE     0x82000001
#define GNEE_HVC_CPU_PIN         0x82000002
#define GNEE_HVC_GPU_SUBMIT      0x82000003
#define GNEE_HVC_THERMAL_SET     0x82000004

/* Rooted HAL context */
struct gnee_hal_rooted {
    struct device *dev;
    
    /* Hypervisor state */
    struct kvm *kvm;
    struct kvm_vm *vm;
    u32 vmid;
    
    /* Memory management */
    struct ion_client *ion_client;
    struct dma_buf *shared_dma_buf;
    
    /* CPU state */
    cpumask_t pinned_cpus;
    bool thermal_heartbeat_active;
    
    /* GPU state */
    struct drm_device *gpu_drm;
    struct drm_context *gpu_context;
    
    /* Interrupt state */
    struct irq_desc *irq_descs[256];
    
    /* Flags */
    u32 flags;
    
    /* Statistics */
    struct {
        u64 memory_allocations;
        u64 memory_frees;
        u64 cpu_pins;
        u64 gpu_submissions;
        u64 interrupt_count;
    } stats;
};

/* Function prototypes */
int gnee_hal_rooted_init(struct gnee_hal_rooted *hal);
void gnee_hal_rooted_cleanup(struct gnee_hal_rooted *hal);

/* Memory management */
int gnee_hal_rooted_alloc_memory(struct gnee_hal_rooted *hal, size_t size,
                                 phys_addr_t *phys, void **virt);
int gnee_hal_rooted_free_memory(struct gnee_hal_rooted *hal, phys_addr_t phys);
int gnee_hal_rooted_map_physical(struct gnee_hal_rooted *hal, phys_addr_t phys,
                                 size_t size, void **virt);

/* CPU control */
int gnee_hal_rooted_pin_cpu(struct gnee_hal_rooted *hal, u32 core_id);
int gnee_hal_rooted_set_priority(struct gnee_hal_rooted *hal, int priority);

/* GPU control */
int gnee_hal_rooted_submit_gpu(struct gnee_hal_rooted *hal,
                               struct drm_gem_object *gem_obj);

/* Thermal management */
int gnee_hal_rooted_start_thermal(struct gnee_hal_rooted *hal);
int gnee_hal_rooted_stop_thermal(struct gnee_hal_rooted *hal);

/* Hypervisor calls */
static inline int gnee_hvc_call(u32 func, u64 arg0, u64 arg1)
{
    int ret;
    u64 val;
    
    asm volatile(
        "mov x0, %1\n"
        "mov x1, %2\n"
        "mov x2, %3\n"
        "hvc #0\n"
        "mov %0, x0\n"
        : "=r"(ret)
        : "r"(func), "r"(arg0), "r"(arg1)
        : "x0", "x1", "x2", "memory"
    );
    
    return ret;
}

#endif /* _GNEE_HAL_ROOTED_H_ */
