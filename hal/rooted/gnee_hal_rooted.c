/* SPDX-License-Identifier: GPL-2.0 */
/*
 * Game-Native Execution Environment - Rooted HAL Implementation
 *
 * Copyright (C) 2025 GNEE Team
 */

#include "gnee_hal_rooted.h"
#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/init.h>
#include <linux/fs.h>
#include <linux/uaccess.h>
#include <linux/slab.h>
#include <linux/dma-mapping.h>
#include <linux/platform_device.h>
#include <linux/of.h>
#include <linux/interrupt.h>
#include <linux/thermal.h>
#include <soc/qcom/smem.h>

/* Module information */
MODULE_AUTHOR("GNEE Team");
MODULE_LICENSE("GPL");
MODULE_DESCRIPTION("GNEE Rooted HAL - Hypervisor-level hardware access");

/* Global HAL instance */
static struct gnee_hal_rooted *g_hal_instance;

/*
 * Initialize rooted HAL
 */
int gnee_hal_rooted_init(struct gnee_hal_rooted *hal)
{
    int ret;
    
    if (!hal)
        return -EINVAL;
    
    memset(hal, 0, sizeof(*hal));
    
    /* Check for KVM support */
    if (!kvm_arch_supported()) {
        pr_err("GNEE: KVM not supported on this platform\n");
        return -ENODEV;
    }
    
    /* Initialize ION client */
    hal->ion_client = ion_client_create("gnee-rooted");
    if (IS_ERR(hal->ion_client)) {
        pr_err("GNEE: Failed to create ION client\n");
        ret = PTR_ERR(hal->ion_client);
        goto err;
    }
    
    /* Create VM */
    hal->vm = kvm_create_vm();
    if (IS_ERR(hal->vm)) {
        pr_err("GNEE: Failed to create VM\n");
        ret = PTR_ERR(hal->vm);
        goto err_ion;
    }
    
    hal->vmid = hal->vm->vmid;
    hal->flags |= GNEE_HAL_ROOTED_FLAG_HYPERVISOR;
    
    /* Check SMMU support */
    if (iommu_get_domain_for_dev(hal->dev)) {
        hal->flags |= GNEE_HAL_ROOTED_FLAG_SMMU;
    }
    
    /* Check ION support */
    hal->flags |= GNEE_HAL_ROOTED_FLAG_ION;
    
    pr_info("GNEE Rooted HAL initialized (VMID: %u)\n", hal->vmid);
    pr_info("GNEE Features: %s%s%s\n",
            (hal->flags & GNEE_HAL_ROOTED_FLAG_HYPERVISOR) ? "Hypervisor " : "",
            (hal->flags & GNEE_HAL_ROOTED_FLAG_SMMU) ? "SMMU " : "",
            (hal->flags & GNEE_HAL_ROOTED_FLAG_ION) ? "ION " : "");
    
    return 0;

err_ion:
    ion_client_destroy(hal->ion_client);
err:
    return ret;
}
EXPORT_SYMBOL(gnee_hal_rooted_init);

/*
 * Cleanup rooted HAL
 */
void gnee_hal_rooted_cleanup(struct gnee_hal_rooted *hal)
{
    if (!hal)
        return;
    
    /* Stop thermal heartbeat */
    if (hal->thermal_heartbeat_active)
        gnee_hal_rooted_stop_thermal(hal);
    
    /* Free VM */
    if (hal->vm)
        kvm_put_vm(hal->vm);
    
    /* Destroy ION client */
    if (hal->ion_client)
        ion_client_destroy(hal->ion_client);
    
    pr_info("GNEE Rooted HAL cleanup complete\n");
}
EXPORT_SYMBOL(gnee_hal_rooted_cleanup);

/*
 * Allocate pinned memory via hypervisor
 */
int gnee_hal_rooted_alloc_memory(struct gnee_hal_rooted *hal, size_t size,
                                 phys_addr_t *phys, void **virt)
{
    struct ion_handle *handle;
    struct dma_buf *dmabuf;
    void *vaddr;
    phys_addr_t paddr;
    int ret;
    
    if (!hal || !hal->ion_client)
        return -EINVAL;
    
    /* Allocate via ION */
    handle = ion_alloc(hal->ion_client, size, 0,
                       ION_HEAP(ION_SYSTEM_HEAP_ID), 0);
    if (IS_ERR(handle)) {
        ret = PTR_ERR(handle);
        pr_err("GNEE: ION allocation failed: %d\n", ret);
        goto err;
    }
    
    /* Export as dma-buf */
    dmabuf = ion_share_dma_buf(hal->ion_client, handle);
    if (IS_ERR(dmabuf)) {
        ret = PTR_ERR(dmabuf);
        goto err_handle;
    }
    
    /* Map for CPU access */
    vaddr = dma_buf_vmap(dmabuf);
    if (!vaddr) {
        ret = -ENOMEM;
        goto err_dmabuf;
    }
    
    /* Get physical address */
    paddr = dma_buf_phys(dmabuf);
    
    /* Pin via hypervisor */
    ret = gnee_hvc_call(GNEE_HVC_MEMORY_ALLOC, paddr, size);
    if (ret < 0) {
        pr_err("GNEE: Hypervisor memory pin failed: %d\n", ret);
        goto err_vmap;
    }
    
    *phys = paddr;
    *virt = vaddr;
    
    hal->stats.memory_allocations++;
    
    return 0;

err_vmap:
    dma_buf_vunmap(dmabuf, vaddr);
err_dmabuf:
    dma_buf_put(dmabuf);
err_handle:
    ion_free(hal->ion_client, handle);
err:
    return ret;
}
EXPORT_SYMBOL(gnee_hal_rooted_alloc_memory);

/*
 * Free pinned memory
 */
int gnee_hal_rooted_free_memory(struct gnee_hal_rooted *hal, phys_addr_t phys)
{
    int ret;
    
    if (!hal)
        return -EINVAL;
    
    /* Unpin via hypervisor */
    ret = gnee_hvc_call(GNEE_HVC_MEMORY_FREE, phys, 0);
    if (ret < 0) {
        pr_err("GNEE: Hypervisor memory free failed: %d\n", ret);
        return ret;
    }
    
    hal->stats.memory_frees++;
    
    return 0;
}
EXPORT_SYMBOL(gnee_hal_rooted_free_memory);

/*
 * Map physical address to virtual
 */
int gnee_hal_rooted_map_physical(struct gnee_hal_rooted *hal, phys_addr_t phys,
                                 size_t size, void **virt)
{
    void *vaddr;
    
    if (!hal || !virt)
        return -EINVAL;
    
    /* Use ioremap for device memory */
    vaddr = ioremap(phys, size);
    if (!vaddr)
        return -ENOMEM;
    
    *virt = vaddr;
    return 0;
}
EXPORT_SYMBOL(gnee_hal_rooted_map_physical);

/*
 * Pin CPU to specific core
 */
int gnee_hal_rooted_pin_cpu(struct gnee_hal_rooted *hal, u32 core_id)
{
    struct task_struct *task = current;
    cpumask_t mask;
    int ret;
    
    if (!hal)
        return -EINVAL;
    
    if (core_id >= nr_cpu_ids)
        return -EINVAL;
    
    cpumask_clear(&mask);
    cpumask_set_cpu(core_id, &mask);
    
    ret = sched_setaffinity(task->pid, &mask);
    if (ret < 0) {
        pr_err("GNEE: Failed to pin CPU to core %u: %d\n", core_id, ret);
        return ret;
    }
    
    cpumask_set_cpu(core_id, &hal->pinned_cpus);
    hal->stats.cpu_pins++;
    
    pr_debug("GNEE: Pinned to core %u\n", core_id);
    
    return 0;
}
EXPORT_SYMBOL(gnee_hal_rooted_pin_cpu);

/*
 * Set thread priority
 */
int gnee_hal_rooted_set_priority(struct gnee_hal_rooted *hal, int priority)
{
    struct task_struct *task = current;
    struct sched_param param;
    int policy;
    
    if (!hal)
        return -EINVAL;
    
    param.sched_priority = priority;
    
    if (priority > 0) {
        policy = SCHED_FIFO;
    } else {
        policy = SCHED_NORMAL;
        param.sched_priority = 0;
    }
    
    return sched_setscheduler(task, policy, &param);
}
EXPORT_SYMBOL(gnee_hal_rooted_set_priority);

/*
 * Submit GPU command
 */
int gnee_hal_rooted_submit_gpu(struct gnee_hal_rooted *hal,
                               struct drm_gem_object *gem_obj)
{
    if (!hal || !gem_obj)
        return -EINVAL;
    
    if (!hal->gpu_drm)
        return -ENODEV;
    
    /* Submit via hypervisor for direct SMMU access */
    return gnee_hvc_call(GNEE_HVC_GPU_SUBMIT, (u64)gem_obj, 0);
}
EXPORT_SYMBOL(gnee_hal_rooted_submit_gpu);

/*
 * Start thermal heartbeat
 */
int gnee_hal_rooted_start_thermal(struct gnee_hal_rooted *hal)
{
    if (!hal)
        return -EINVAL;
    
    if (hal->thermal_heartbeat_active)
        return 0;
    
    /* Enable thermal heartbeat via hypervisor */
    gnee_hvc_call(GNEE_HVC_THERMAL_SET, 1, 0);
    
    hal->thermal_heartbeat_active = true;
    pr_info("GNEE: Thermal heartbeat started\n");
    
    return 0;
}
EXPORT_SYMBOL(gnee_hal_rooted_start_thermal);

/*
 * Stop thermal heartbeat
 */
int gnee_hal_rooted_stop_thermal(struct gnee_hal_rooted *hal)
{
    if (!hal)
        return -EINVAL;
    
    if (!hal->thermal_heartbeat_active)
        return 0;
    
    /* Disable thermal heartbeat */
    gnee_hvc_call(GNEE_HVC_THERMAL_SET, 0, 0);
    
    hal->thermal_heartbeat_active = false;
    pr_info("GNEE: Thermal heartbeat stopped\n");
    
    return 0;
}
EXPORT_SYMBOL(gnee_hal_rooted_stop_thermal);

/*
 * Module init
 */
static int __init gnee_hal_rooted_module_init(void)
{
    pr_info("GNEE Rooted HAL module loaded\n");
    return 0;
}

/*
 * Module exit
 */
static void __exit gnee_hal_rooted_module_exit(void)
{
    pr_info("GNEE Rooted HAL module unloaded\n");
}

module_init(gnee_hal_rooted_module_init);
module_exit(gnee_hal_rooted_module_exit);
