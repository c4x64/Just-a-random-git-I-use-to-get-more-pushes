// SPDX-License-Identifier: GPL-2.0
/*
 * Game-Native Execution Environment - JIT Emitter
 *
 * Copyright (C) 2025 GNEE Team
 *
 * ARM64 instruction emitter for the JIT compiler.
 */

//! # JIT Emitter
//!
//! Emits ARM64 instructions from an intermediate representation.

use crate::error::{Error, Result};
use crate::jit::decoder::{Register, Opcode, Instruction};

/// ARM64 emitter
pub struct Emitter {
    /// Code buffer
    buffer: Vec<u8>,
    /// Current position
    position: usize,
    /// Labels
    labels: Vec<Label>,
    /// Fixups
    fixups: Vec<Fixup>,
}

/// Label
#[derive(Debug, Clone)]
pub struct Label {
    pub name: String,
    pub position: usize,
}

/// Fixup
#[derive(Debug, Clone)]
pub struct Fixup {
    pub label: String,
    pub position: usize,
    pub type_: FixupType,
}

/// Fixup type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixupType {
    BranchOffset,
    LoadLiteral,
    Address,
}

impl Emitter {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            position: 0,
            labels: Vec::new(),
            fixups: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            position: 0,
            labels: Vec::new(),
            fixups: Vec::new(),
        }
    }

    pub fn emit_mov_reg(&mut self, rd: Register, rm: Register) {
        let rd = self.reg(rd) as u32;
        let rm = self.reg(rm) as u32;
        let instr = 0x2A000000u32 | rm | (rd << 5);
        self.emit_u32(instr);
    }

    pub fn emit_add(&mut self, rd: Register, rn: Register, rm: Register) {
        let rd = self.reg(rd) as u32;
        let rn = self.reg(rn) as u32;
        let rm = self.reg(rm) as u32;
        let instr = 0x0B000000u32 | rm | (rn << 5) | (rd << 10);
        self.emit_u32(instr);
    }

    pub fn emit_sub(&mut self, rd: Register, rn: Register, rm: Register) {
        let rd = self.reg(rd) as u32;
        let rn = self.reg(rn) as u32;
        let rm = self.reg(rm) as u32;
        let instr = 0x4B000000u32 | rm | (rn << 5) | (rd << 10);
        self.emit_u32(instr);
    }

    pub fn emit_and(&mut self, rd: Register, rn: Register, rm: Register) {
        let rd = self.reg(rd) as u32;
        let rn = self.reg(rn) as u32;
        let rm = self.reg(rm) as u32;
        let instr = 0x0A000000u32 | rm | (rn << 5) | (rd << 10);
        self.emit_u32(instr);
    }

    pub fn emit_orr(&mut self, rd: Register, rn: Register, rm: Register) {
        let rd = self.reg(rd) as u32;
        let rn = self.reg(rn) as u32;
        let rm = self.reg(rm) as u32;
        let instr = 0x2A000000u32 | rm | (rn << 5) | (rd << 10);
        self.emit_u32(instr);
    }

    pub fn emit_eor(&mut self, rd: Register, rn: Register, rm: Register) {
        let rd = self.reg(rd) as u32;
        let rn = self.reg(rn) as u32;
        let rm = self.reg(rm) as u32;
        let instr = 0x4A000000u32 | rm | (rn << 5) | (rd << 10);
        self.emit_u32(instr);
    }

    pub fn emit_ldr(&mut self, rd: Register, rn: Register, offset: i32) {
        let rd = self.reg(rd) as u32;
        let rn = self.reg(rn) as u32;
        let imm12 = ((offset >> 2) & 0xFFF) as u32;
        let instr = 0x39400000u32 | (imm12 << 10) | (rn << 5) | rd;
        self.emit_u32(instr);
    }

    pub fn emit_str(&mut self, rd: Register, rn: Register, offset: i32) {
        let rd = self.reg(rd) as u32;
        let rn = self.reg(rn) as u32;
        let imm12 = ((offset >> 2) & 0xFFF) as u32;
        let instr = 0x39000000u32 | (imm12 << 10) | (rn << 5) | rd;
        self.emit_u32(instr);
    }

    pub fn emit_bl(&mut self, offset: i32) {
        let imm26 = ((offset >> 2) & 0x3FFFFFF) as u32;
        let instr = 0x94000000u32 | imm26;
        self.emit_u32(instr);
    }

    pub fn emit_br(&mut self, rn: Register) {
        let rn = self.reg(rn) as u32;
        let instr = 0xD61F0000u32 | (rn << 5);
        self.emit_u32(instr);
    }

    pub fn emit_ret(&mut self) {
        self.emit_u32(0xD65F03C0u32);
    }

    pub fn emit_nop(&mut self) {
        self.emit_u32(0xD503201Fu32);
    }

    pub fn emit_dmb(&mut self, option: u8) {
        let instr = 0xD5000000u32 | ((option as u32) << 8) | 0xF000u32;
        self.emit_u32(instr);
    }

    pub fn emit_isb(&mut self) {
        self.emit_u32(0xD503205Fu32);
    }

    pub fn emit_label(&mut self, name: &str) {
        self.labels.push(Label {
            name: name.to_string(),
            position: self.position,
        });
    }

    pub fn emit_fixup(&mut self, label: &str, type_: FixupType) {
        self.fixups.push(Fixup {
            label: label.to_string(),
            position: self.position,
            type_,
        });
        self.emit_u32(0);
    }

    pub fn resolve_fixups(&mut self) -> Result<()> {
        for fixup in &self.fixups.clone() {
            let label = self.labels.iter()
                .find(|l| l.name == fixup.label)
                .ok_or(Error::EmitterError("Label not found"))?;

            let offset = (label.position - fixup.position - 8) as i32;
            let instr = self.read_u32(fixup.position);
            let patched = self.patch_branch(instr, offset);
            self.write_u32(fixup.position, patched);
        }
        Ok(())
    }

    fn patch_branch(&self, instr: u32, offset: i32) -> u32 {
        let imm26 = ((offset >> 2) & 0x3FFFFFF) as u32;
        instr & !0x3FFFFFFu32 | imm26
    }

    fn reg(&self, r: Register) -> u8 {
        match r {
            Register::Rax | Register::R0 => 0,
            Register::Rcx | Register::R1 => 1,
            Register::Rdx | Register::R2 => 2,
            Register::Rbx | Register::R3 => 3,
            Register::Rsp | Register::R4 => 4,
            Register::Rbp | Register::R5 => 5,
            Register::Rsi | Register::R6 => 6,
            Register::Rdi | Register::R7 => 7,
            Register::R8 => 8,
            Register::R9 => 9,
            Register::R10 => 10,
            Register::R11 => 11,
            Register::R12 => 12,
            Register::R13 => 13,
            Register::R14 => 14,
            Register::R15 => 15,
            _ => 0,
        }
    }

    fn emit_u32(&mut self, value: u32) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
        self.position += 4;
    }

    fn read_u32(&self, offset: usize) -> u32 {
        let bytes = &self.buffer[offset..offset + 4];
        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
    }

    fn write_u32(&mut self, offset: usize, value: u32) {
        let bytes = value.to_le_bytes();
        self.buffer[offset..offset + 4].copy_from_slice(&bytes);
    }

    pub fn code(&self) -> &[u8] { &self.buffer }
    pub fn code_size(&self) -> usize { self.buffer.len() }
    pub fn position(&self) -> usize { self.position }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.position = 0;
        self.labels.clear();
        self.fixups.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emitter_creation() {
        let emitter = Emitter::new();
        assert_eq!(emitter.code_size(), 0);
    }

    #[test]
    fn test_emit_nop() {
        let mut emitter = Emitter::new();
        emitter.emit_nop();
        assert_eq!(emitter.code_size(), 4);
    }

    #[test]
    fn test_emit_ret() {
        let mut emitter = Emitter::new();
        emitter.emit_ret();
        assert_eq!(emitter.code_size(), 4);
    }
}
