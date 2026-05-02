// SPDX-License-Identifier: GPL-2.0
/*
 * Game-Native Execution Environment - JIT Decoder
 *
 * Copyright (C) 2025 GNEE Team
 *
 * x86-64 instruction decoder for the JIT compiler.
 */

//! # JIT Decoder
//!
//! Decodes x86-64 instructions into an intermediate representation.

use crate::error::{Error, Result};

/// x86-64 instruction decoder
pub struct Decoder {
    /// Current position in the code stream
    position: usize,
    /// Code stream
    code: Vec<u8>,
    /// Current instruction
    current_instruction: Option<Instruction>,
}

/// x86-64 instruction
#[derive(Debug, Clone, PartialEq)]
pub struct Instruction {
    /// Opcode
    pub opcode: Opcode,
    /// Operands
    pub operands: Vec<Operand>,
    /// Instruction size in bytes
    pub size: u8,
    /// Prefixes
    pub prefixes: Prefixes,
    /// REX prefix
    pub rex: Option<Rex>,
}

/// x86-64 opcode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
    // Data transfer
    Mov,
    Movsx,
    Movzx,
    Lea,
    Push,
    Pop,
    
    // Arithmetic
    Add,
    Sub,
    Mul,
    Imul,
    Div,
    Idiv,
    Inc,
    Dec,
    Neg,
    
    // Logical
    And,
    Or,
    Xor,
    Not,
    
    // Shift/Rotate
    Shl,
    Shr,
    Sar,
    Rol,
    Ror,
    
    // Control flow
    Jmp,
    Call,
    Ret,
    Je,
    Jne,
    Jg,
    Jl,
    Jge,
    Jle,
    Jo,
    Jno,
    Js,
    Jns,
    
    // Conditional moves
    Cmov,
    
    // String operations
    Movs,
    Cmps,
    Scas,
    Lods,
    Stos,
    
    // System
    Syscall,
    Sysenter,
    Sysexit,
    
    // SSE/AVX
    Movd,
    Movq,
    Movdqa,
    Movdqu,
    Paddb,
    Paddw,
    Paddd,
    Psubb,
    Psubw,
    Psubd,
    Pmullw,
    Pmuludq,
    
    // FPU
    Fadd,
    Fsub,
    Fmul,
    Fdiv,
    
    // Other
    Nop,
    Ud2,
    Invalid,
}

/// Operand types
#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    /// Register operand
    Register(Register),
    /// Immediate value
    Immediate(i64),
    /// Memory operand
    Memory(MemoryOperand),
    /// Relative offset
    Relative(i32),
    /// Absolute address
    Absolute(u64),
}

/// x86-64 registers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Register {
    // General purpose
    Rax, Rcx, Rdx, Rbx, Rsp, Rbp, Rsi, Rdi,
    R8, R9, R10, R11, R12, R13, R14, R15,
    
    // 32-bit
    Eax, Ecx, Edx, Ebx, Esp, Ebp, Esi, Edi,
    R8d, R9d, R10d, R11d, R12d, R13d, R14d, R15d,
    
    // 16-bit
    Ax, Cx, Dx, Bx, Sp, Bp, Si, Di,
    R8w, R9w, R10w, R11w, R12w, R13w, R14w, R15w,
    
    // 8-bit
    Al, Cl, Dl, Bl, Ah, Ch, Dh, Bh,
    R8b, R9b, R10b, R11b, R12b, R13b, R14b, R15b,
    
    // Segment registers
    Es, Cs, Ss, Ds, Fs, Gs,
    
    // Control registers
    Cr0, Cr2, Cr3, Cr4, Cr8,
    
    // Debug registers
    Dr0, Dr1, Dr2, Dr3, Dr4, Dr5, Dr6, Dr7,
    
    // XMM registers
    Xmm0, Xmm1, Xmm2, Xmm3, Xmm4, Xmm5, Xmm6, Xmm7,
    Xmm8, Xmm9, Xmm10, Xmm11, Xmm12, Xmm13, Xmm14, Xmm15,
    
    // YMM registers
    Ymm0, Ymm1, Ymm2, Ymm3, Ymm4, Ymm5, Ymm6, Ymm7,
    Ymm8, Ymm9, Ymm10, Ymm11, Ymm12, Ymm13, Ymm14, Ymm15,
    
    // RIP
    Rip,
}

/// Memory operand
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryOperand {
    /// Base register
    pub base: Option<Register>,
    /// Index register
    pub index: Option<Register>,
    /// Scale factor
    pub scale: u8,
    /// Displacement
    pub displacement: i32,
    /// Segment override
    pub segment: Option<Register>,
}

/// Instruction prefixes
#[derive(Debug, Clone, Copy, Default)]
pub struct Prefixes {
    /// Lock prefix
    pub lock: bool,
    /// Repeat prefix
    pub rep: bool,
    /// Repeat not zero prefix
    pub repne: bool,
    /// Segment override
    pub segment: Option<Register>,
    /// Operand size override
    pub operand_size: bool,
    /// Address size override
    pub address_size: bool,
}

/// REX prefix
#[derive(Debug, Clone, Copy, Default)]
pub struct Rex {
    /// 64-bit operand size
    pub w: bool,
    /// Extension to ModR/M reg field
    pub r: bool,
    /// Extension to SIB index field
    pub x: bool,
    /// Extension to ModR/M r/m field
    pub b: bool,
}

impl Decoder {
    /// Create a new decoder
    pub fn new(code: Vec<u8>) -> Self {
        Self {
            position: 0,
            code,
            current_instruction: None,
        }
    }

    /// Decode next instruction
    pub fn decode(&mut self) -> Result<Instruction> {
        if self.position >= self.code.len() {
            return Err(Error::DecoderError("End of code stream"));
        }

        let start_pos = self.position;
        let mut prefixes = Prefixes::default();
        let mut rex = None;

        // Decode prefixes
        self.decode_prefixes(&mut prefixes, &mut rex)?;

        // Decode opcode
        let opcode = self.decode_opcode()?;

        // Decode operands
        let operands = self.decode_operands(&opcode, &rex)?;

        let size = (self.position - start_pos) as u8;

        let instruction = Instruction {
            opcode,
            operands,
            size,
            prefixes,
            rex,
        };

        self.current_instruction = Some(instruction.clone());
        Ok(instruction)
    }

    /// Decode instruction prefixes
    fn decode_prefixes(&mut self, prefixes: &mut Prefixes, rex: &mut Option<Rex>) -> Result<()> {
        loop {
            if self.position >= self.code.len() {
                break;
            }

            let byte = self.code[self.position];

            match byte {
                0xF0 => prefixes.lock = true,
                0xF3 => prefixes.rep = true,
                0xF2 => prefixes.repne = true,
                0x2E => prefixes.segment = Some(Register::Cs),
                0x36 => prefixes.segment = Some(Register::Ss),
                0x3E => prefixes.segment = Some(Register::Ds),
                0x26 => prefixes.segment = Some(Register::Es),
                0x64 => prefixes.segment = Some(Register::Fs),
                0x65 => prefixes.segment = Some(Register::Gs),
                0x66 => prefixes.operand_size = true,
                0x67 => prefixes.address_size = true,
                0x40..=0x4F => {
                    // REX prefix
                    *rex = Some(Rex {
                        w: (byte & 0x8) != 0,
                        r: (byte & 0x4) != 0,
                        x: (byte & 0x2) != 0,
                        b: (byte & 0x1) != 0,
                    });
                }
                _ => break,
            }

            self.position += 1;
        }

        Ok(())
    }

    /// Decode opcode
    fn decode_opcode(&mut self) -> Result<Opcode> {
        if self.position >= self.code.len() {
            return Err(Error::DecoderError("End of code stream"));
        }

        let byte = self.code[self.position];
        self.position += 1;

        match byte {
            // Data transfer
            0x88..=0x8B => Ok(Opcode::Mov),
            0x8D => Ok(Opcode::Lea),
            0x50..=0x57 | 0x41..=0x47 => Ok(Opcode::Push),
            0x58..=0x5F | 0x48..=0x4F => Ok(Opcode::Pop),
            
            // Arithmetic
            0x01..=0x03 => Ok(Opcode::Add),
            0x29..=0x2B => Ok(Opcode::Sub),
            0xF7 => {
                // Need to check ModR/M byte
                Ok(Opcode::Mul)
            }
            0x40..=0x47 => Ok(Opcode::Inc),
            0x48..=0x4F => Ok(Opcode::Dec),
            
            // Logical
            0x21..=0x23 => Ok(Opcode::And),
            0x09..=0x0B => Ok(Opcode::Or),
            0x31..=0x33 => Ok(Opcode::Xor),
            
            // Control flow
            0xE8 => Ok(Opcode::Call),
            0xC3 => Ok(Opcode::Ret),
            0x74 => Ok(Opcode::Je),
            0x75 => Ok(Opcode::Jne),
            0x7F => Ok(Opcode::Jg),
            0x7C => Ok(Opcode::Jl),
            0x7D => Ok(Opcode::Jge),
            0x7E => Ok(Opcode::Jle),
            0x70 => Ok(Opcode::Jo),
            0x71 => Ok(Opcode::Jno),
            0x78 => Ok(Opcode::Js),
            0x79 => Ok(Opcode::Jns),
            
            // System
            0x0F => self.decode_two_byte_opcode(),
            
            // Other
            0x90 => Ok(Opcode::Nop),
            0x0B => Ok(Opcode::Ud2),
            
            _ => Ok(Opcode::Invalid),
        }
    }

    /// Decode two-byte opcode
    fn decode_two_byte_opcode(&mut self) -> Result<Opcode> {
        if self.position >= self.code.len() {
            return Err(Error::DecoderError("End of code stream"));
        }

        let byte = self.code[self.position];
        self.position += 1;

        match byte {
            0x05 => Ok(Opcode::Syscall),
            0x34 => Ok(Opcode::Sysenter),
            0x35 => Ok(Opcode::Sysexit),
            0x10..=0x15 => Ok(Opcode::Movups),
            0x28..=0x2F => Ok(Opcode::Movaps),
            0x6F => Ok(Opcode::Movdqa),
            0x7F => Ok(Opcode::Movdqa),
            _ => Ok(Opcode::Invalid),
        }
    }

    /// Decode operands
    fn decode_operands(&mut self, opcode: &Opcode, rex: &Option<Rex>) -> Result<Vec<Operand>> {
        let mut operands = Vec::new();

        // This is a simplified operand decoder
        // A full implementation would handle all addressing modes
        
        match opcode {
            Opcode::Mov | Opcode::Add | Opcode::Sub | Opcode::And | Opcode::Or | Opcode::Xor => {
                // These instructions typically have 2 operands
                operands.push(Operand::Register(Register::Rax));
                operands.push(Operand::Register(Register::Rbx));
            }
            Opcode::Push | Opcode::Pop => {
                operands.push(Operand::Register(Register::Rsp));
            }
            Opcode::Call => {
                operands.push(Operand::Relative(0));
            }
            Opcode::Jmp => {
                operands.push(Operand::Relative(0));
            }
            Opcode::Ret => {
                // No operands
            }
            _ => {
                // Default: no operands
            }
        }

        Ok(operands)
    }

    /// Get current position
    pub fn position(&self) -> usize {
        self.position
    }

    /// Get remaining bytes
    pub fn remaining(&self) -> usize {
        self.code.len() - self.position
    }

    /// Reset decoder to beginning
    pub fn reset(&mut self) {
        self.position = 0;
        self.current_instruction = None;
    }

    /// Seek to position
    pub fn seek(&mut self, position: usize) -> Result<()> {
        if position > self.code.len() {
            return Err(Error::DecoderError("Invalid position"));
        }
        self.position = position;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decoder_creation() {
        let code = vec![0x90, 0x90, 0x90];
        let decoder = Decoder::new(code);
        assert_eq!(decoder.position(), 0);
        assert_eq!(decoder.remaining(), 3);
    }

    #[test]
    fn test_nop_decode() {
        let code = vec![0x90];
        let mut decoder = Decoder::new(code);
        let instruction = decoder.decode().unwrap();
        assert_eq!(instruction.opcode, Opcode::Nop);
        assert_eq!(instruction.size, 1);
    }

    #[test]
    fn test_mov_decode() {
        let code = vec![0x48, 0x89, 0xD8]; // mov rax, rbx
        let mut decoder = Decoder::new(code);
        let instruction = decoder.decode().unwrap();
        assert_eq!(instruction.opcode, Opcode::Mov);
    }

    #[test]
    fn test_rex_prefix() {
        let code = vec![0x48, 0x89, 0xD8];
        let mut decoder = Decoder::new(code);
        let instruction = decoder.decode().unwrap();
        assert!(instruction.rex.is_some());
        assert!(instruction.rex.unwrap().w);
    }
}
