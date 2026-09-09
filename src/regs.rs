
use core::marker::ConstParamTy;
use std::cmp::max;

use emu_lib::{alu::AluFlags, bitfield::{BitfieldBase, BitfieldField, BitfieldFieldLength}, bitfield, datatypes::{LeU8, LeU32}, regs::{Regfile, RegfileDesc, RegfileView}};

use bytemuck::{CheckedBitPattern, NoUninit, Pod, Zeroable};

use crate::{except::SkyarchException, intr::{Intctl, Intret, Inttab}};

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, ConstParamTy, CheckedBitPattern, NoUninit)]
#[repr(u8)]
pub enum Map {
    Gprs = 0,
    Intr = 1,
    Io = 2,
    Info = 3,
    CoprocessorControl = 4,
    _Reserved5 = 5,
    _Reserved6 = 6,
    _Reserved7 = 7,
    Coprocessor0 = 8,
    Coprocessor1 = 9,
    Coprocessor2 = 10,
    Coprocessor3 = 11,
    Coprocessor4 = 12,
    Coprocessor5 = 13,
    Coprocessor6 = 14,
    Coprocessor7 = 15,
}

const impl BitfieldFieldLength for Map {
    fn max_field_width() -> u32 {
        4
    }
}

const impl<R: [const] BitfieldBase> BitfieldField<R> for Map where LeU8: [const] BitfieldField<R> {
    fn decode(val: R) -> Self {
        let val = LeU8::decode(val).to_ne();
        assert!(val < 16);

        unsafe { core::mem::transmute(val) }
    }

    fn encode(self) -> R {
        let val: LeU8 = bytemuck::must_cast(self);

        val.encode()
    }
}

#[derive(Copy, Clone, Hash, PartialEq, Eq, NoUninit, CheckedBitPattern)]
#[repr(u8)]
enum SkyarchRegnoInner {
    _00 = 0,
    _01 = 1,
    _02 = 2,
    _03 = 3,
    _04 = 4,
    _05 = 5,
    _06 = 6,
    _07 = 7,
    _08 = 8,
    _09 = 9,
    _0A = 10,
    _0B = 11,
    _0C = 12,
    _0D = 13,
    _0E = 14,
    _0F = 15,
    _10 = 16,
    _11 = 17,
    _12 = 18,
    _13 = 19,
    _14 = 20,
    _15 = 21,
    _16 = 22,
    _17 = 23,
    _18 = 24,
    _19 = 25,
    _1A = 26,
    _1B = 27,
    _1C = 28,
    _1D = 29,
    _1E = 30,
    _1F = 31,
}

#[derive(Copy, Clone, Hash, PartialEq, Eq, NoUninit, CheckedBitPattern)]
#[repr(transparent)]
pub struct SkyarchRegno(SkyarchRegnoInner);

impl SkyarchRegno {

    pub const ZERO: Self = Self(SkyarchRegnoInner::_00);

    pub const fn get(self) -> u8 {
        self.0 as u8
    }
}

impl core::fmt::Debug for SkyarchRegno {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("r{}", self.0 as u8))
    }
}

const impl BitfieldFieldLength for SkyarchRegno {
    fn max_field_width() -> u32 {
        5
    }
}

const impl<R: [const] BitfieldBase> BitfieldField<R> for SkyarchRegno where LeU8: [const] BitfieldField<R> {
    fn decode(val: R) -> Self {
        let val = LeU8::decode(val).to_ne();
        assert!(val < 32);

        unsafe { core::mem::transmute(val) }
    }

    fn encode(self) -> R {
        let val: LeU8 = bytemuck::must_cast(self);

        val.encode()
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct SkyarchRegs<const M: Map>;

impl<const M: Map> RegfileDesc for SkyarchRegs<M> {
    type Reg = LeU32;

    const LEN: usize = 32;
}

impl RegfileView for SkyarchRegs<{Map::Intr}> {
    type View = SkyarchIntrMap;
}

impl RegfileView for SkyarchRegs<{Map::Io}> {
    type View = IoRegs;
}

impl RegfileView for SkyarchRegs<{Map::CoprocessorControl}> {
    type View = CoprocessorCtl;
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Pod, Zeroable)]
#[repr(C)]
pub struct SkyarchIntrMap {
    pub intctl: Intctl,
    pub intret: [Intret; 3],
    pub ints: [LeU32; 4],
    pub intd: [LeU32; 4],
    __reserved: [LeU32; 19],
    pub inttab: Inttab,
}


#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Pod, Zeroable)]
#[repr(C)]
pub struct CoprocessorCtl {
    pub ctlregs: [LeU32; 8],
    __reserved: [LeU32; 22],
    pub coprocessors_enabled: LeU32,
    pub coprocessors_available: LeU32,
}


#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Pod, Zeroable)]
#[repr(transparent)]
pub struct SkyarchRegisters {
    flat_regs: [[LeU32; 32]; 16],
}

impl SkyarchRegisters {
    pub const fn new() -> Self {
        Self { flat_regs: bytemuck::zeroed() }
    }

    pub const fn reset(&mut self, coprocessors: u8) {
        emu_lib::ops::write_zeroes(self);
        self.coctl_mut().coprocessors_available = LeU8::from_ne(coprocessors).extend();
    }

    pub const fn map<const M: Map>(&self) -> &Regfile<SkyarchRegs<M>> {
        unsafe {&*(&raw const self.flat_regs[M as usize] as *const _)}
    }

    pub const fn map_mut<const M: Map>(&mut self) -> &mut Regfile<SkyarchRegs<M>> {
        unsafe {&mut *(&raw mut self.flat_regs[M as usize] as *mut _)}
    }

    pub const fn intr(&self) -> &SkyarchIntrMap {
        self.map::<{Map::Intr}>().regs_view()
    }

    pub const fn intr_mut(&mut self) -> &mut SkyarchIntrMap {
        self.map_mut::<{Map::Intr}>().regs_view_mut()
    }

    pub const fn coctl(&self) -> &CoprocessorCtl {
        self.map::<{Map::CoprocessorControl}>().regs_view()
    }

    pub const fn io(&mut self) -> &mut IoRegs {
        self.map_mut::<{Map::Io}>().regs_view_mut()
    }

    pub const fn coctl_mut(&mut self) -> &mut CoprocessorCtl {
        self.map_mut::<{Map::CoprocessorControl}>().regs_view_mut()
    }

    const fn raw_read(&self, map: Map, reg: SkyarchRegno) -> LeU32 {
        self.flat_regs[map as usize][reg.get() as usize]
    }

    const fn raw_write(&mut self, map: Map, reg: SkyarchRegno, val: LeU32) {
        self.flat_regs[map as usize][reg.get() as usize] = val;
    }

    pub const fn write_gpr(&mut self, reg: SkyarchRegno, val: LeU32) {
        let reg = reg.get() as usize;
        if reg != 0 {
            self.flat_regs[0][reg] = val;
        }
    }

    pub const fn coprocessor_enabled(&self, n: u8) -> Result<(), SkyarchException> {
        assert!(n < 8);

        if (self.coctl().coprocessors_enabled.to_ne() & (1 << (n as u32))) == 0 {
            Err(SkyarchException::Undefined)
        } else {
            Ok(())
        }
    }

    pub const fn read_gpr(&self, reg: SkyarchRegno) -> LeU32 {
        self.flat_regs[0][reg.get() as usize]
    }

    pub const fn check_write(&self, map: Map, reg: SkyarchRegno) -> Result<(), SkyarchException> {
        match map {
            Map::Gprs | Map::Io => Ok(()),
            Map::Intr => {
                match reg.get() {
                    0..12 | 31 => Ok(()),
                    _ => Err(SkyarchException::Undefined)
                }
            },
            
            Map::CoprocessorControl => {
                match reg.get() {
                    n @ (0..8) => self.coprocessor_enabled(n),
                    30 => Ok(()),
                    _ => Err(SkyarchException::Undefined),
                }
            },
            Map::Info |
            Map::_Reserved5 |
            Map::_Reserved6 |
            Map::_Reserved7 => Err(SkyarchException::Undefined),
            n => {
                let n = (n as u8).strict_sub(8);
                self.coprocessor_enabled(n)
            }
        }
    }

    pub const fn write(&mut self, map: Map, reg: SkyarchRegno, val: LeU32) -> Result<(), SkyarchException> {
        match map {
            Map::Gprs => {
                self.write_gpr(reg, val);
                Ok(())
            },
            Map::Intr => {
                match reg.get() {
                    1..4 | 4..12 => {
                        self.raw_write(map, reg, val);
                        Ok(())
                    }
                    0 => {
                        if (val & (0x8000_0003)) != val {
                            Err(SkyarchException::Consistency)
                        } else {
                            self.raw_write(map, reg, val);
                            Ok(())
                        }
                    }
                    31 => {
                        if (val & !0x7) != val {
                            Err(SkyarchException::Consistency)
                        } else {
                            self.raw_write(map, reg, val);
                            Ok(())
                        }
                    }
                    _ => Err(SkyarchException::Undefined)
                }
            },
            Map::Io => {
                self.raw_write(map, reg,val);
                Ok(())
            },
            
            Map::CoprocessorControl => {
                match reg.get() {
                    x @ (0..8) => {
                        self.coprocessor_enabled(x)?;

                        self.raw_write(map, reg, val);
                        Ok(())
                    }
                    30 => {
                        let avail = self.coctl().coprocessors_available;

                        if (val & avail) != val {
                            Err(SkyarchException::Consistency)
                        } else {
                            self.raw_write(map, reg, val);
                            Ok(())
                        }
                    }
                    _ => Err(SkyarchException::Undefined)
                }
            },
            Map::Info |
            Map::_Reserved5 |
            Map::_Reserved6 |
            Map::_Reserved7 => Err(SkyarchException::Undefined),
            Map::Coprocessor0 => todo!(),
            Map::Coprocessor1 => todo!(),
            Map::Coprocessor2 => todo!(),
            Map::Coprocessor3 => todo!(),
            Map::Coprocessor4 => todo!(),
            Map::Coprocessor5 => todo!(),
            Map::Coprocessor6 => todo!(),
            Map::Coprocessor7 => todo!(),
        }
    } 

    pub const fn read(&self, map: Map, reg: SkyarchRegno) -> Result<LeU32, SkyarchException> {
        match map {
            Map::Io => Ok(self.raw_read(map, reg)),
            Map::Intr => {
                match reg.get() {
                    0..12 | 31 => Ok(self.raw_read(map, reg)),
                    _ => Err(SkyarchException::Undefined)
                }
            }
            Map::Info | Map::Gprs => Ok(LeU32::ZERO),
            Map::CoprocessorControl => {
                match reg.get() {
                    x @ (0..8) => {
                        self.coprocessor_enabled(x)?;
                        Ok(self.raw_read(map, reg))
                    }
                    30 | 31 => Ok(self.raw_read(map, reg)),
                    _ => Err(SkyarchException::Undefined)
                }
            }
            Map::_Reserved5 | Map::_Reserved6 | Map::_Reserved7 => Err(SkyarchException::Undefined),
            n => {
                let x = (n as u8).strict_sub(8);
                self.coprocessor_enabled(x)?;
                Ok(self.raw_read(map, reg))
            }
        }
    }
}

bitfield!{
    #[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Pod, Zeroable)]
    pub struct SkyarchFlags : LeU32 {
        pub c @ 0: bool,
        pub v @ 1: bool,
        pub n @ 2: bool,
        pub z @ 3: bool,
        pub p @ 4: bool,
    }
}


impl SkyarchFlags {
    pub fn xvp(&mut self) {
        let parity = self.p();
        self.set_p(self.v());
        self.set_v(parity);
    }
}

impl AluFlags for SkyarchFlags {
    fn set_carry(&mut self, carry: bool) {
        self.set_c(carry);
    }

    fn set_overflow(&mut self, overflow: bool) {
        self.set_v(overflow);
    }

    fn set_logic(&mut self, zero: bool, sign: bool, parity: bool) {
        self.set_z(zero);
        self.set_n(sign);
        self.set_p(parity);
    }
}

#[derive(Copy, Clone, Hash, PartialEq, Eq, Zeroable, Pod)]
#[repr(transparent)]
pub struct ShiftRegister(LeU32);

impl ShiftRegister {
    pub fn shift(&mut self, val: LeU32, bits: u32) -> LeU32 {
        if bits == 32 {
            let res = self.0;
            self.0 = val;
            return res;
        }
        let out = 0u32.funnel_shl(self.0.to_ne(), bits);
        let val = self.0.to_ne().funnel_shl(val.to_ne(), bits);
        LeU32::from_ne(out)
    }
}

#[derive(Copy, Clone, Hash, PartialEq, Eq, Zeroable, Pod)]
#[repr(C)]
pub struct IoRegs {
    pub io: [ShiftRegister; 32],
}