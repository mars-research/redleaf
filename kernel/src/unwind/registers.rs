// From unwind-rs/registers.rs

use core::fmt::{Debug, Formatter, Result as FmtResult};
use core::ops::{Index, IndexMut};

#[derive(Default, Clone, PartialEq, Eq)]
pub struct Registers {
    registers: [Option<u64>; 17],
}

impl Debug for Registers {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        for reg in &self.registers {
            match *reg {
                None => write!(fmt, " XXX")?,
                Some(x) => write!(fmt, " 0x{:x}", x)?,
            }
        }
        Ok(())
    }
}

impl Index<u16> for Registers {
    type Output = Option<u64>;

    fn index(&self, index: u16) -> &Option<u64> {
        &self.registers[index as usize]
    }
}

impl IndexMut<u16> for Registers {
    fn index_mut(&mut self, index: u16) -> &mut Option<u64> {
        &mut self.registers[index as usize]
    }
}

impl Index<gimli::Register> for Registers {
    type Output = Option<u64>;

    fn index(&self, reg: gimli::Register) -> &Option<u64> {
        &self[reg.0]
    }
}

impl IndexMut<gimli::Register> for Registers {
    fn index_mut(&mut self, reg: gimli::Register) -> &mut Option<u64> {
        &mut self[reg.0]
    }
}

// From Theseus unwinder

/// Contains the registers that are callee-saved.
/// This is intended to be used at the beginning of stack unwinding for two purposes:
/// 1. The unwinding tables need an initial value for these registers in order to 
///    calculate the register values for the previous stack frame based on register transformation rules,
/// 2. To know which register values to restore after unwinding is complete.
/// 
/// This is currently x86_64-specific.
#[derive(Debug)]
#[repr(C)]
pub struct SavedRegs {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbx: u64,
    pub rbp: u64,
}

/// Contains the register values that will be restored to the actual CPU registers
/// right before jumping to a landing pad function.
/// 
/// # Important Note
/// This should be kept in sync with the number of elements 
/// in the `Registers` struct; this must have one less element.
#[derive(Debug)]
#[repr(C)]
pub struct LandingRegisters {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rbp: u64,
    pub r8:  u64,
    pub r9:  u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rsp: u64,
    // Not sure if we need to include other registers here, like rflags or segment registers. 
    // We probably do for SIMD at least.
}
