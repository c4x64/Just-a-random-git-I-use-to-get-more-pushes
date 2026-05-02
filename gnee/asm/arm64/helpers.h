/* SPDX-License-Identifier: GPL-2.0 */
/*
 * Game-Native Execution Environment - ARM64 Assembly Helpers
 *
 * Copyright (C) 2025 GNEE Team
 *
 * ARM64 assembly helpers for hypervisor entry/exit and memory management.
 */

#ifndef _GNEE_ASM_ARM64_HELPERS_H_
#define _GNEE_ASM_ARM64_HELPERS_H_

#include <linux/linkage.h>
#include <asm/assembler.h>

/*
 * Hypervisor entry point
 * Called from EL1 via HVC instruction
 */
.macro hyp_entry
    .align 7
    stp     x29, x30, [sp, #-16]!
    mov     x29, sp

    /* Save all caller-saved registers */
    stp     x0, x1, [sp, #-16]!
    stp     x2, x3, [sp, #-16]!
    stp     x4, x5, [sp, #-16]!
    stp     x6, x7, [sp, #-16]!
    stp     x8, x9, [sp, #-16]!
    stp     x10, x11, [sp, #-16]!
    stp     x12, x13, [sp, #-16]!
    stp     x14, x15, [sp, #-16]!

    /* Read ESR_EL2 for exit classification */
    mrs     x0, esr_el2
    mrs     x1, elr_el2
    mrs     x2, far_el2

    /* Branch to C handler */
    bl      kvm_hyp_handler

    /* Restore all caller-saved registers */
    ldp     x14, x15, [sp], #16
    ldp     x12, x13, [sp], #16
    ldp     x10, x11, [sp], #16
    ldp     x8, x9, [sp], #16
    ldp     x6, x7, [sp], #16
    ldp     x4, x5, [sp], #16
    ldp     x2, x3, [sp], #16
    ldp     x0, x1, [sp], #16

    ldp     x29, x30, [sp], #16
    eret
.endm

/*
 * Stage-2 MMU setup
 */
.macro stage2_mmu_setup
    mrs     x0, vtcr_el2
    bfc     x0, #0, #6, x1
    bfi     x0, #2, #14, #2
    bfi     x0, #3, #12, #2
    bfi     x0, #1, #8, #2
    bfi     x0, #1, #10, #2
    msr     vtcr_el2, x0
    isb
.endm

/*
 * Memory barrier macros
 */
.macro dmbsy
    dmb     sy
.endm

.macro dmbsld
    dmb     ld
.endm

.macro dmbsst
    dmb     st
.endm

.macro isb_all
    isb
.endm

/*
 * Cache maintenance operations
 */
.macro dc_ival xreg
    dc      ivac, \xreg
.endm

.macro dc_cival xreg
    dc      civac, \xreg
.endm

.macro dc_csw xreg
    dc      csw, \xreg
.endm

/*
 * SMMU context switch
 */
.macro smmu_context_switch
    msr     vttbr_el2, x0
    isb
    msr     contextidr_el2, x1
    isb
    tlbi    vmalls12e1is
    dsb     sy
    isb
.endm

/*
 * Interrupt mask/unmask
 */
.macro interrupt_mask
    msr     daifset, #0xF
    isb
.endm

.macro interrupt_unmask
    msr     daifclr, #0xF
    isb
.endm

/*
 * Get current CPU ID
 */
.macro get_cpu_id
    mrs     x0, mpidr_el1
    and     x0, x0, #0xFF
.endm

/*
 * Counter-timer access
 */
.macro get_cntvct xreg
    mrs     \xreg, cntvct_el0
.endm

.macro get_cntfrq xreg
    mrs     \xreg, cntfrq_el0
.endm

/*
 * Guest register context save/restore
 */
.macro save_guest_context xctx
    stp     x0, x1, [\xctx, #0x00]
    stp     x2, x3, [\xctx, #0x10]
    stp     x4, x5, [\xctx, #0x20]
    stp     x6, x7, [\xctx, #0x30]
    stp     x8, x9, [\xctx, #0x40]
    stp     x10, x11, [\xctx, #0x50]
    stp     x12, x13, [\xctx, #0x60]
    stp     x14, x15, [\xctx, #0x70]
    stp     x16, x17, [\xctx, #0x80]
    stp     x18, x19, [\xctx, #0x90]
    stp     x20, x21, [\xctx, #0xA0]
    stp     x22, x23, [\xctx, #0xB0]
    stp     x24, x25, [\xctx, #0xC0]
    stp     x26, x27, [\xctx, #0xD0]
    stp     x28, x29, [\xctx, #0xE0]
    stp     x30, sp, [\xctx, #0xF0]
    mrs     x0, spsr_el2
    str     x0, [\xctx, #0x100]
    mrs     x0, elr_el2
    str     x0, [\xctx, #0x108]
.endm

.macro restore_guest_context xctx
    ldp     x0, x1, [\xctx, #0x00]
    ldp     x2, x3, [\xctx, #0x10]
    ldp     x4, x5, [\xctx, #0x20]
    ldp     x6, x7, [\xctx, #0x30]
    ldp     x8, x9, [\xctx, #0x40]
    ldp     x10, x11, [\xctx, #0x50]
    ldp     x12, x13, [\xctx, #0x60]
    ldp     x14, x15, [\xctx, #0x70]
    ldp     x16, x17, [\xctx, #0x80]
    ldp     x18, x19, [\xctx, #0x90]
    ldp     x20, x21, [\xctx, #0xA0]
    ldp     x22, x23, [\xctx, #0xB0]
    ldp     x24, x25, [\xctx, #0xC0]
    ldp     x26, x27, [\xctx, #0xD0]
    ldp     x28, x29, [\xctx, #0xE0]
    ldp     x30, sp, [\xctx, #0xF0]
    ldr     x0, [\xctx, #0x100]
    msr     spsr_el2, x0
    ldr     x0, [\xctx, #0x108]
    msr     elr_el2, x0
.endm

/*
 * Performance counters
 */
.macro pmc_start
    mov     x0, #0x8000000F
    msr     pmcr_el0, x0
    isb
.endm

.macro pmc_stop
    mov     x0, #0
    msr     pmcr_el0, x0
    isb
.endm

.macro pmc_read xreg
    mrs     \xreg, pmccntr_el0
.endm

#endif /* _GNEE_ASM_ARM64_HELPERS_H_ */
