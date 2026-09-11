
use bytemuck::{Pod, Zeroable};

use emu_lib::bitfield::{BitfieldBase, BitfieldField, BitfieldFieldLength};
use emu_lib::datatypes::*;

use crate::except::SkyarchException;

use crate::regs::{Map, SkyarchFlags, SkyarchRegisters, SkyarchRegno};

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
#[repr(u8)]
pub enum ConditionCode {
    Never = 0,
    Carry = 1,
    Zero = 2,
    Overflow = 3,
    CarryOrEqual = 4,
    SignedLess = 5,
    SignedLessOrEq = 6,
    Negative = 7,
    Positive = 8,
    SignedGreater = 9,
    SignedGreaterOrEq = 10,
    Above = 11,
    NotOverflow = 12,
    NotZero = 13,
    NotCarry = 14,
    Always = 15
}

impl ConditionCode {
    pub fn check_condition(&self, flags: SkyarchFlags) -> bool {
        match self {
            ConditionCode::Never => false,
            ConditionCode::Carry => flags.c(),
            ConditionCode::Zero => flags.z(),
            ConditionCode::Overflow => flags.v(),
            ConditionCode::CarryOrEqual => flags.c() | flags.z(),
            ConditionCode::SignedLess => (flags.c() == flags.n()) & !flags.z(),
            ConditionCode::SignedLessOrEq => flags.c() == flags.n(),
            ConditionCode::Negative => flags.n(),
            ConditionCode::Positive => !flags.n(),
            ConditionCode::SignedGreater => flags.c() != flags.n(),
            ConditionCode::SignedGreaterOrEq => (flags.c() != flags.n()) | flags.z(),
            ConditionCode::Above => !flags.c() & !flags.z(),
            ConditionCode::NotOverflow => !flags.v(),
            ConditionCode::NotZero => !flags.z(),
            ConditionCode::NotCarry => !flags.c(),
            ConditionCode::Always => true,
        }
    }
}

const impl BitfieldFieldLength for ConditionCode {
    fn max_field_width() -> u32 {
        4
    }
}

const impl<B: [const] BitfieldBase> BitfieldField<B> for ConditionCode where LeU8: [const] BitfieldField<B> {
    fn decode(val: B) -> Self {
        let val = LeU8::decode(val).to_ne();

        assert!(val < 16);

        unsafe { core::mem::transmute(val) }
    }

    fn encode(self) -> B {
        LeU8::from_ne(self as u8).encode()
    }
}

macro_rules! skyarch_instr {
    (
        $(#[doc $($edoc:tt)*])*
        $vis:vis enum $enum:ident {
            $(
                $(#[doc $($idoc:tt)*])*
                $instr:ident {$($fname:ident @ $fbase:literal$(..$fend:literal)?: $fty:ty),* $(,)?} = $opc:literal $(/ $lbits:literal)?
            ),+
            $(,)?
        }
    ) => {
        $(#[doc $($edoc:tt)*])*
        #[repr(u8)]
        $vis enum $enum {
            $(
                $(#[doc $($idoc:tt)*])*
                $instr (${concat($instr, Payload)}) = $opc,
            )*
        }

        impl $enum {
            pub fn decode(w: LeU32) -> Result<Self, SkyarchException> {
                let opcode = w.to_le_bytes::<4>()[0];

                match opcode {
                    $($(x if x & !(1 << $lbits) ==)? $opc => {
                        let payload: ${concat($instr, Payload)} = bytemuck::must_cast(w);

                        if !payload.is_valid() {
                            Err(SkyarchException::Undefined)
                        } else {
                            Ok(Self::$instr (payload))
                        }

                    })*
                    _ => Err(SkyarchException::Undefined)
                }
            }
        }

        $(
            ::emu_lib::bitfield! {
                $(#[doc $($idoc:tt)*])*
                #[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Pod, Zeroable)]
                $vis struct ${concat($instr, Payload)} : LeU32 {
                    __opcode @ 0..8: LeU8,
                    $(pub $fname @ $fbase $(..$fend)?: $fty),*
                }
            }
        )*
    };
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
#[repr(u8)]
pub enum UpdateMode {
    None = 0,
    PostInc = 1,
    _Reserved = 2,
    PreDec = 3,
}

const impl BitfieldFieldLength for UpdateMode {
    fn max_field_width() -> u32 {
        2
    }
}

const impl<B: [const] BitfieldBase> BitfieldField<B> for UpdateMode where LeU8: [const] BitfieldField<B> {
    fn decode(val: B) -> Self {
        let val = LeU8::decode(val).to_ne();

        assert!(val < 4);

        unsafe { core::mem::transmute(val) }
    }

    fn encode(self) -> B {
        LeU8::from_ne(self as u8).encode()
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
#[repr(u8)]
pub enum Ordering {
    Relaxed = 0,
    Acquire = 1,
    Release = 2,
    SeqCst = 3,
}

const impl BitfieldFieldLength for Ordering {
    fn max_field_width() -> u32 {
        2
    }
}

const impl<B: [const] BitfieldBase> BitfieldField<B> for Ordering where LeU8: [const] BitfieldField<B> {
    fn decode(val: B) -> Self {
        let val = LeU8::decode(val).to_ne();

        assert!(val < 4);

        unsafe { core::mem::transmute(val) }
    }

    fn encode(self) -> B {
        LeU8::from_ne(self as u8).encode()
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
#[repr(u8)]
pub enum MemWidth {
    Byte = 0,
    Half = 1,
    Word = 2,
    Double = 3,
}

impl MemWidth {
    pub const fn amask(&self) -> LeU32 {
        match self {
            MemWidth::Byte => LeU32::from_ne(0),
            MemWidth::Half => LeU32::from_ne(1),
            MemWidth::Word => LeU32::from_ne(3),
            MemWidth::Double => LeU32::from_ne(7),
        }
    }
}

const impl BitfieldFieldLength for MemWidth {
    fn max_field_width() -> u32 {
        2
    }
}

const impl<B: [const] BitfieldBase> BitfieldField<B> for MemWidth where LeU8: [const] BitfieldField<B> {
    fn decode(val: B) -> Self {
        let val = LeU8::decode(val).to_ne();

        assert!(val < 4);

        unsafe { core::mem::transmute(val) }
    }

    fn encode(self) -> B {
        LeU8::from_ne(self as u8).encode()
    }
}

skyarch_instr! {
    pub enum SkyarchInstr {
        Und00 {ing @ 8..32: LeU32} = 0x00,
        UndFF {ing @ 8..32: LeU32} = 0xFF,
        Pause {k @ 8..14: LeU8} = 0x01,
        Mov {dest @ 8..13: SkyarchRegno, cc @ 13..17: ConditionCode, l @ 17: bool, src @ 18..23: SkyarchRegno, dir @ 25: bool, map @ 26..30: Map} = 0x02,
        St {dest @ 8..13: SkyarchRegno, src @ 13..18: SkyarchRegno, width @ 18..20: MemWidth, mode @ 28..30: UpdateMode, order @ 30..32: Ordering} = 0x03,
        Ld {dest @ 8..13: SkyarchRegno, src @ 13..18: SkyarchRegno, width @ 18..20: MemWidth, mode @ 28..30: UpdateMode, order @ 30..32: Ordering} = 0x04,
        Ldi {dest @ 8..13: SkyarchRegno, x @ 13: bool, i @ 16..32: LeU16} = 0x05,
        Lra {dest @ 8..13: SkyarchRegno, x @ 13: bool, o @ 16..32: LeU16} = 0x06,
        Addi {dest @ 8..13: SkyarchRegno, x @ 13: bool, f @ 14: bool, h @ 15: bool, i @ 16..32: LeU16} = 0x08,
        Add { dest @ 8..13: SkyarchRegno, a @ 13..18: SkyarchRegno, b @ 18..23: SkyarchRegno, f @ 23: bool, s @ 24..29: LeU8, p @ 29: bool, c @ 31: bool} = 0x09,
        Sub { dest @ 8..13: SkyarchRegno, a @ 13..18: SkyarchRegno, b @ 18..23: SkyarchRegno, f @ 23: bool, s @ 24..29: LeU8, p @ 29: bool, c @ 31: bool} = 0x0A,
        And { dest @ 8..13: SkyarchRegno, a @ 13..18: SkyarchRegno, b @ 18..23: SkyarchRegno, f @ 23: bool, s @ 24..29: LeU8, p @ 29: bool, i @ 30: bool, j @ 31: bool} = 0x0B,
        Or { dest @ 8..13: SkyarchRegno, a @ 13..18: SkyarchRegno, b @ 18..23: SkyarchRegno, f @ 23: bool, s @ 24..29: LeU8, p @ 29: bool, i @ 30: bool, j @ 31: bool} = 0x0C,
        Xor { dest @ 8..13: SkyarchRegno, a @ 13..18: SkyarchRegno, b @ 18..23: SkyarchRegno, f @ 23: bool, s @ 24..29: LeU8, p @ 29: bool, i @ 30: bool, j @ 31: bool} = 0x0D,
        Fsl { dest @ 8..13: SkyarchRegno, v @ 13..18: SkyarchRegno, q @ 18..23: SkyarchRegno, f @ 23: bool, x @ 24: bool, w @ 26: bool, r @ 27..32: SkyarchRegno} = 0x0E,
        Fsr { dest @ 8..13: SkyarchRegno, v @ 13..18: SkyarchRegno, q @ 18..23: SkyarchRegno, f @ 23: bool, x @ 24: bool, w @ 26: bool, r @ 27..32: SkyarchRegno} = 0x0F,

        Jmp {l @ 8..13: SkyarchRegno, cc @ 13..17: ConditionCode, off @ 17..31: LeU16, offx @ 31: bool} = 0x10,
        Jmpr {l @ 8..13: SkyarchRegno, cc @ 13..17: ConditionCode, r @ 18..23: SkyarchRegno} = 0x11,
        Iret {p @ 18..20: LeU8} = 0x12,

        In {d @ 8..13: SkyarchRegno, p @ 13..21: LeU8, w @ 27..32: LeU32} = 0x14,
        Out {s @ 8..13: SkyarchRegno, p @ 13..21: LeU8, w @ 27..32: LeU32} = 0x15,

        Ldflags {d @ 8..13: SkyarchRegno, fmask @ 13..18: LeU8} = 0x18,
        Stflags {s @ 8..13: SkyarchRegno, fmask @ 13..18: LeU8} = 0x19,
        Xvp {} = 0x1A,
        Xchg {a @ 8..13: SkyarchRegno, cc @ 13..17: ConditionCode, l @ 17: bool, b @ 18..23: SkyarchRegno} = 0x1C,
        Ext { dest @ 8..13: SkyarchRegno, src @ 13..18: SkyarchRegno, x @ 18: bool, width @ 27..32: LeU32} = 0x1D,
        Bswap { dest @ 8..13: SkyarchRegno, src @ 13..18: SkyarchRegno } = 0x1E,
        Rbgen {dest @ 8..13: SkyarchRegno, err @ 18..23: SkyarchRegno, f @ 23: bool, width @ 27..32: LeU32} = 0x1F,

        Cpi {f @ 8..12: LeU8, p @ 12..32: LeU32} = 0x20 / 4,
        CpiEf {f @ 8..14: LeU8, p @ 14..32: LeU32} = 0x30 / 4,

        Halt {m @ 8..10: LeU8} = 0x40,
        Breakpoint {} = 0x41,
        Fence { rr @ 30..32: Ordering} = 0x48,
        Stic {dest @ 8..13: SkyarchRegno, src @ 13..18: SkyarchRegno, width @ 18..20: MemWidth, f @ 23: bool, order @ 30..32: Ordering} = 0x4B,
        Ldil {dest @ 8..13: SkyarchRegno, src @ 13..18: SkyarchRegno, width @ 18..20: MemWidth, order @ 30..32: Ordering} = 0x4C,
        Sticw {dest @ 8..13: SkyarchRegno, src @ 13..18: SkyarchRegno, src2 @ 18..23: SkyarchRegno, f @ 23: bool, order @ 30..32: Ordering} = 0x4D,
        Ldilw {dest @ 8..13: SkyarchRegno, src @ 13..18: SkyarchRegno, dest2 @ 18..23: SkyarchRegno, order @ 30..32: Ordering} = 0x4E,
    }
}

impl CpiPayload {
    pub const fn coprocessor(&self) -> LeU8 {
        self.__opcode() & 0x3
    }

    pub const fn nonblocking(&self) -> bool {
        (self.__opcode() & 0x4) != LeU8::zero()
    }
}

impl CpiEfPayload {
    pub const fn coprocessor(&self) -> LeU8 {
        self.__opcode() & 0x3
    }

    pub const fn nonblocking(&self) -> bool {
        (self.__opcode() & 0x4) != LeU8::zero()
    }
}

impl SkyarchInstr {
    pub fn validate(&self, regs: &SkyarchRegisters) -> Result<(), SkyarchException> {
        match self {
            SkyarchInstr::Und00(_) |
            SkyarchInstr::UndFF(_) => Err(SkyarchException::Undefined),
            SkyarchInstr::St(st) => {
                if st.dest() == SkyarchRegno::ZERO {
                    Err(SkyarchException::Undefined)
                } else if st.order() == Ordering::Acquire {
                    Err(SkyarchException::Undefined)
                } else if st.mode() == UpdateMode::_Reserved {
                    Err(SkyarchException::Undefined)
                } else if st.width() == MemWidth::Double {
                    Err(SkyarchException::Undefined)
                } else {
                    Ok(())
                }
            },
            SkyarchInstr::Ld(ld) => {
                if ld.src() == SkyarchRegno::ZERO {
                    Err(SkyarchException::Undefined)
                } else if ld.order() == Ordering::Release {
                    Err(SkyarchException::Undefined)
                } else if ld.mode() == UpdateMode::_Reserved {
                    Err(SkyarchException::Undefined)
                } else if ld.width() == MemWidth::Double {
                    Err(SkyarchException::Undefined)
                } else {
                    Ok(())
                }
            },
            SkyarchInstr::Addi(addi) => {
                if addi.h() & addi.x() {
                    Err(SkyarchException::Undefined)
                } else {
                    Ok(())
                }
            },
            SkyarchInstr::Jmpr(jmpr) => {
                if jmpr.r() == SkyarchRegno::ZERO {
                    Err(SkyarchException::Undefined)
                } else {
                    Ok(())
                }
            },
            SkyarchInstr::Iret(iret) => {
                if iret.p() == LeInt::zero() {
                    Err(SkyarchException::Undefined)
                } else {
                    Ok(())
                }
            },
            SkyarchInstr::Ext(ext) => {
                if ext.width() == LeInt::zero() {
                    Err(SkyarchException::Undefined)
                } else {
                    Ok(())
                }
            },
            SkyarchInstr::Rbgen(rbgen) => {
                if !rbgen.f() {
                    Err(SkyarchException::Undefined)
                } else {
                    Ok(())
                }
            }
            SkyarchInstr::Cpi(cpi) => regs.coprocessor_enabled(cpi.coprocessor().to_ne()),
            SkyarchInstr::CpiEf(cpief) => regs.coprocessor_enabled(cpief.coprocessor().to_ne()),
            SkyarchInstr::Fence(fence) => {
                if fence.rr() == Ordering::Relaxed {
                    Err(SkyarchException::Undefined)
                } else {
                    Ok(())
                }
            },
            SkyarchInstr::Stic(st) => {
                if st.dest() == SkyarchRegno::ZERO {
                    Err(SkyarchException::Undefined)
                } else if st.order() == Ordering::Acquire {
                    Err(SkyarchException::Undefined)
                } else if st.width() == MemWidth::Double {
                    Err(SkyarchException::Undefined)
                } else if !st.f() {
                    Err(SkyarchException::Undefined)
                } else {
                    Ok(())
                }
            },
            SkyarchInstr::Sticw(st) => {
                if st.dest() == SkyarchRegno::ZERO {
                    Err(SkyarchException::Undefined)
                } else if st.order() == Ordering::Acquire {
                    Err(SkyarchException::Undefined)
                } else if !st.f() {
                    Err(SkyarchException::Undefined)
                } else {
                    Ok(())
                }
            },
            SkyarchInstr::Ldil(ld) => {
                if ld.src() == SkyarchRegno::ZERO {
                    Err(SkyarchException::Undefined)
                } else if ld.order() == Ordering::Release {
                    Err(SkyarchException::Undefined)
                } else if ld.width() == MemWidth::Double {
                    Err(SkyarchException::Undefined)
                } else {
                    Ok(())
                }
            },
            SkyarchInstr::Ldilw(ld) => {
                if ld.src() == SkyarchRegno::ZERO {
                    Err(SkyarchException::Undefined)
                } else if ld.order() == Ordering::Release {
                    Err(SkyarchException::Undefined)
                } else {
                    Ok(())
                }
            },
            _ => Ok(())
        }
    }
}