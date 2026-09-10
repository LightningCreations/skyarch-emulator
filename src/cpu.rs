use emu_lib::{alu::{self, AluFlags, arith_op}, bitfield::AlignedAddr, cpu::{CpuDef, NoVirt, Status}, datatypes::{LeInt, LeU8, LeU16, LeU32}};

use crate::{except::SkyarchException, instr::{MemWidth, SkyarchInstr}, intr::{IntSlot, Intctl, Intret}, regs::{Map, SkyarchFlags, SkyarchRegisters}};


fn extend(val: LeU16, x: bool) -> LeU32 {
    val.extend::<u32>() | (if x {
        LeU32::from_ne(0xFFFF_0000)
    } else { LeU32::zero()})
}

fn dyn_extend(val: LeU32, width: u32, sign_ext: bool) -> LeU32 {
    let mask = ((val & ((sign_ext as u32) << (width - 1))) << (31 - width)) - 1;

    (val & ((1 << width) - 1)) | (mask << width)
}

pub struct Skyarch {
    active_mask: LeU8,
    il_addr: LeU32,
    il_width: Option<MemWidth>,
}

impl Skyarch {
    pub const fn new() -> Self {
        Self {
            active_mask: LeU8::from_ne(0),
            il_addr: LeU32::zero(),
            il_width: None,
        }
    }
}

impl CpuDef for Skyarch {
    type VAddr = LeU32;

    type PAddr= LeU32;

    type IoAddress = LeU8;

    type InstrWord = LeU32;

    type IoData = LeU32;

    type MmioData = LeU32;

    type CacheLine = [LeU32; 16];

    type Exception = SkyarchException;

    type DecodedInstruction = SkyarchInstr;

    type Registers = SkyarchRegisters;

    type Flags = SkyarchFlags;

    type AddrResolver = NoVirt<LeU32, SkyarchException>;

    const MIN_PAGE_SIZE: usize = 4096;

    fn reset<M: emu_lib::memory::MemorySupplier<Self::PAddr, Data = Self::CacheLine>>(cpu: &mut emu_lib::cpu::Cpu<Self, M>) {
        cpu.registers_mut().reset(0);
        let _ = cpu.do_jump(LeU32::from_ne(0xFF00));
    }

    fn decode<M: emu_lib::memory::MemorySupplier<Self::PAddr, Data = Self::CacheLine>, I: IntoIterator<Item = Result<Self::InstrWord, Self::Exception>>>(cpu: &emu_lib::cpu::Cpu<Self, M>, instr: I) -> Result<Self::DecodedInstruction, Self::Exception> {
        let mut instr = instr.into_iter();
        let val = instr.next().unwrap()?;

        let op = SkyarchInstr::decode(val)?;

        op.validate(cpu.registers())?;

        Ok(op)
    }

    fn execute<M: emu_lib::memory::MemorySupplier<Self::PAddr, Data = Self::CacheLine>>(cpu: &mut emu_lib::cpu::Cpu<Self, M>, instr: Self::DecodedInstruction, cur_ip: Self::VAddr) -> Result<Option<Self::VAddr>, Self::Exception> {
        match instr {
            SkyarchInstr::Und00(_) |
            SkyarchInstr::UndFF(_) => panic!("Should not decode"),
            SkyarchInstr::Pause(_) => {},
            SkyarchInstr::Mov(mov) => {
                let dest_map = if mov.dir() {
                    mov.map()
                } else {
                    Map::Gprs
                };

                let src_map = if mov.dir() {
                    Map::Gprs
                } else {
                    mov.map()
                };

                let dest_reg = mov.dest();
                let src_reg = mov.src();

                let val = cpu.registers().read(src_map, src_reg)?;

                cpu.registers().check_write(dest_map, dest_reg)?;

                if mov.cc().check_condition(*cpu.flags()) {
                    cpu.registers_mut().write(dest_map, mov.dest(), val)?;
                }
            },
            SkyarchInstr::St(st) => {
                let addr_reg = st.dest();
                let src = st.src();

                let addr = cpu.registers().read_gpr(addr_reg);

                let width = st.width();

                let val = cpu.registers().read_gpr(src);

                if (addr & width.amask()) != LeU32::zero() {
                    return Err(SkyarchException::BusError)
                }


                match width {
                    crate::instr::MemWidth::Byte => cpu.write(addr, val.truncate::<u8>())?,
                    crate::instr::MemWidth::Half => cpu.write(addr, val.truncate::<u16>())?,
                    crate::instr::MemWidth::Word => cpu.write(addr, val)?,
                    crate::instr::MemWidth::Double => panic!("64-bit write not allowed (use sticw)"),
                }

            },
            SkyarchInstr::Ld(ld) => {
                let addr_reg = ld.src();
                let dest = ld.dest();

                let addr = cpu.registers().read_gpr(addr_reg);

                let width = ld.width();

                if (addr & width.amask()) != LeU32::zero() {
                    return Err(SkyarchException::BusError)
                }


                let val = match width {
                    crate::instr::MemWidth::Byte => cpu.read::<LeU8>(addr)?.extend(),
                    crate::instr::MemWidth::Half => cpu.read::<LeU16>(addr)?.extend(),
                    crate::instr::MemWidth::Word => cpu.read::<LeU32>(addr)?,
                    crate::instr::MemWidth::Double => panic!("64-bit read not allowed (use ldiw)"),
                };

                cpu.registers_mut().write_gpr(dest, val);
            },
            SkyarchInstr::Ldi(ldi) => {
                let val = extend(ldi.i(), ldi.x());

                let reg = ldi.dest();

                cpu.registers_mut().write_gpr(reg, val);
            },
            SkyarchInstr::Lra(lra) => {
                let val = extend(lra.o(), lra.x()) + cur_ip;

                let reg = lra.dest();

                cpu.registers_mut().write_gpr(reg, val);
            },
            SkyarchInstr::Addi(addi) => {
                let val = extend(addi.i(), addi.x()) << ((addi.h() as u32)*16);

                let reg = addi.dest();

                let in1 = cpu.registers().read_gpr(reg);

                let (res, flags) = arith_op(val, in1, LeInt::zero(), LeU32::overflowing_add);

                if addi.f() {
                    *cpu.flags_mut() = flags;
                }

                cpu.registers_mut().write_gpr(reg, res);
            },
            SkyarchInstr::Add(alu) => {

                let shift1 = if alu.p() { alu.s().to_ne() as u32 } else { 1 };
                let shift2 = if !alu.p() { alu.s().to_ne() as u32 } else { 1 };

                let val1 = cpu.registers().read_gpr(alu.a()) << shift1;
                let val2 = cpu.registers().read_gpr(alu.b()) << shift2;

                let (res, flags) = arith_op(val1, val2, LeInt::from_ne((alu.c() & cpu.flags().c()) as u32), LeU32::overflowing_add);

                if alu.f() {
                    *cpu.flags_mut() = flags;
                }

                cpu.registers_mut().write_gpr(alu.dest(), res)
            },
            SkyarchInstr::Sub(alu) => {
                let shift1 = if alu.p() { alu.s().to_ne() as u32 } else { 1 };
                let shift2 = if !alu.p() { alu.s().to_ne() as u32 } else { 1 };

                let val1 = cpu.registers().read_gpr(alu.a()) << shift1;
                let val2 = cpu.registers().read_gpr(alu.b()) << shift2;
                let (res, flags) = arith_op(val1, val2, LeInt::from_ne(!(alu.c() & cpu.flags().c()) as u32), LeU32::overflowing_sub);

                if alu.f() {
                    *cpu.flags_mut() = flags;
                }

                cpu.registers_mut().write_gpr(alu.dest(), res);
            },
            SkyarchInstr::And(alu) => {
                let shift1 = if alu.p() { alu.s().to_ne() as u32 } else { 1 };
                let shift2 = if !alu.p() { alu.s().to_ne() as u32 } else { 1 };

                let val1 = (cpu.registers().read_gpr(alu.a()) << shift1) ^ (if alu.i() { LeU32::mask() } else { LeU32::zero() });
                let val2 = (cpu.registers().read_gpr(alu.b()) << shift2) ^ (if alu.i() { LeU32::mask() } else { LeU32::zero() });

                let res = val1 & val2;

                if alu.f() {
                    cpu.flags_mut().set_logic_from_val(res);
                }

                cpu.registers_mut().write_gpr(alu.dest(), res);
            },
            SkyarchInstr::Or(alu) => {
                let shift1 = if alu.p() { alu.s().to_ne() as u32 } else { 1 };
                let shift2 = if !alu.p() { alu.s().to_ne() as u32 } else { 1 };

                let val1 = (cpu.registers().read_gpr(alu.a()) << shift1) ^ (if alu.i() { LeU32::mask() } else { LeU32::zero() });
                let val2 = (cpu.registers().read_gpr(alu.b()) << shift2) ^ (if alu.i() { LeU32::mask() } else { LeU32::zero() });

                let res = val1 | val2;

                if alu.f() {
                    cpu.flags_mut().set_logic_from_val(res);
                }

                cpu.registers_mut().write_gpr(alu.dest(), res);
            },
            SkyarchInstr::Xor(alu) => {
                let shift1 = if alu.p() { alu.s().to_ne() as u32 } else { 1 };
                let shift2 = if !alu.p() { alu.s().to_ne() as u32 } else { 1 };

                let val1 = (cpu.registers().read_gpr(alu.a()) << shift1) ^ (if alu.i() { LeU32::mask() } else { LeU32::zero() });
                let val2 = (cpu.registers().read_gpr(alu.b()) << shift2) ^ (if alu.i() { LeU32::mask() } else { LeU32::zero() });

                let res = val1 & val2;

                if alu.f() {
                    cpu.flags_mut().set_logic_from_val(res);
                }

                cpu.registers_mut().write_gpr(alu.dest(), res);
            },
            SkyarchInstr::Fsl(fsl) => {
                let mut val = cpu.registers().read_gpr(fsl.v()).to_ne();
                let quan = cpu.registers().read_gpr(fsl.q()).to_ne();
                let mut rem = cpu.registers().read_gpr(fsl.r()).to_ne();

                if fsl.x() {
                    rem ^=  ((val as i32)>>31) as u32;
                }

                let mut carry = false;
                let mut overflow = false;

                if quan > 32 {
                    overflow = true;
                    if !fsl.w() {
                        if val != 0 {
                            carry =true;
                        }
                        val = rem;
                    }
                }

                let res = LeU32::from_ne(val.funnel_shl(rem, quan));

                if val.unbounded_shr(32-quan) != 0 {
                    carry = true;
                }

                if fsl.f() {
                    let flags = cpu.flags_mut();
                    flags.set_logic_from_val(res);
                    flags.set_carry(carry);
                    flags.set_overflow(overflow);
                }

                cpu.registers_mut().write_gpr(fsl.dest(), res);
            },
            SkyarchInstr::Fsr(fsr) => {
                let mut val = cpu.registers().read_gpr(fsr.v()).to_ne();
                let quan = cpu.registers().read_gpr(fsr.q()).to_ne();
                let mut rem = cpu.registers().read_gpr(fsr.r()).to_ne();

                if fsr.x() {
                    rem ^=  ((val as i32)>>31) as u32;
                }

                let mut carry = false;
                let mut overflow = false;

                if quan > 32 {
                    overflow = true;
                    if !fsr.w() {
                        if val != 0 {
                            carry =true;
                        }
                        val = rem;
                    }
                }

                let res = LeU32::from_ne(val.funnel_shr(rem, quan));

                if val.unbounded_shl(32-quan) != 0 {
                    carry = true;
                }

                if fsr.f() {
                    let flags = cpu.flags_mut();
                    flags.set_logic_from_val(res);
                    flags.set_carry(carry);
                    flags.set_overflow(overflow);
                }

                cpu.registers_mut().write_gpr(fsr.dest(), res);
            },
            SkyarchInstr::Jmp(jmp) => {
                let addr = (extend(jmp.off() << 2u32, jmp.offx())) + cur_ip;

                let link = jmp.l();

                if jmp.cc().check_condition(*cpu.flags()) {
                    cpu.registers_mut().write_gpr(link, cur_ip);
                    return Ok(Some(addr));
                }
            },
            SkyarchInstr::Jmpr(jmpr) => {
                let addr = cpu.registers().read_gpr(jmpr.r());
                if (addr & 3) != LeInt::zero() {
                    return Err(SkyarchException::UnalignedBranch)
                }
                let link = jmpr.l();

                if jmpr.cc().check_condition(*cpu.flags()) {
                    cpu.registers_mut().write_gpr(link, cur_ip);
                    return Ok(Some(addr));
                }
            },
            SkyarchInstr::Iret(iret) => {
                let retreg = iret.p();

                let int = cpu.registers_mut().intr_mut();

                let iret = int.intret[(retreg.to_ne()-1) as usize];

                let addr = iret.addr().into_inner();

                let mask = iret.retmask();

                int.intctl.set_mask(mask);

                return Ok(Some(addr));
            },
            SkyarchInstr::In(inp) => {
                let mut width = inp.w().to_ne();
                if width == 0 {
                    width = 32;
                }
                let bits = cpu.port_in(inp.p(), width);
                cpu.registers_mut().io().io[inp.d().get() as usize].shift(bits, width);
            },
            SkyarchInstr::Out(out) => {
                let mut width = out.w().to_ne();
                if width == 0 {
                    width = 32;
                }
                let bits = cpu.registers_mut().io().io[out.s().get() as usize].shift(LeInt::zero(), width);
                cpu.port_out(out.p(), bits, width);
            },
            SkyarchInstr::Stflags(stflags) => {
                let mask = stflags.fmask();
                let val = cpu.registers().read_gpr(stflags.s()).truncate::<u8>();

                *cpu.flags_mut() = bytemuck::must_cast(val & mask);
            },
            SkyarchInstr::Ldflags(ldflags) => {
                let mask = ldflags.fmask();
                let flags: LeU8 = bytemuck::must_cast(mask);

                cpu.registers_mut().write_gpr(ldflags.d(), (flags & mask).extend())
            },
            SkyarchInstr::Xvp(_) => {
                cpu.flags_mut().xvp();
            },
            SkyarchInstr::Xchg(xchg) => {
                let rval1 = cpu.registers().read_gpr(xchg.a());
                let rval2 = cpu.registers().read_gpr(xchg.b());
                cpu.registers_mut().write_gpr(xchg.a(), rval2);
                cpu.registers_mut().write_gpr(xchg.b(), rval1);
            },
            SkyarchInstr::Ext(ext) => {
                let bits = ext.width().to_ne();

                let val = cpu.registers().read_gpr(ext.src());

                cpu.registers_mut().write_gpr(ext.dest(), dyn_extend(val, bits, ext.x()));
            },
            SkyarchInstr::Bswap(bswap) => {
                let val = cpu.registers().read_gpr(bswap.src());
                let bits = LeU32::from_le_bytes(val.to_be_bytes::<4>());

                cpu.registers_mut().write_gpr(bswap.dest(), bits);
            },
            SkyarchInstr::Rbgen(rbgen) => {
                let bits = if rbgen.width().to_ne() == 0 {32} else { rbgen.width().to_ne() };

                let bytes = cpu.poll_rand::<LeU32>();

                cpu.flags_mut().set_undefined();

                let s = if let Some(bytes) = bytes {
                    cpu.registers_mut().write_gpr(rbgen.dest(), bytes);
                    
                    cpu.flags_mut().set_z(false);

                    0x0_FFFF
                } else {
                    cpu.registers_mut().write_gpr(rbgen.dest(), LeInt::zero());

                    0x3_0000
                };

                let s = LeU32::from_ne(s);

                cpu.registers_mut().write_gpr(rbgen.err(), s);
            },
            SkyarchInstr::Cpi(_) |
            SkyarchInstr::CpiEf(_) => todo!("Coprocessors not supported yet"),
            SkyarchInstr::Halt(hlt) => {
                if hlt.m() == LeInt::zero() {
                    cpu.set_status(Status::Halted);
                } else {
                    cpu.def_status_mut().active_mask = hlt.m();
                    cpu.set_status(Status::Paused)
                }
            },
            SkyarchInstr::Fence(_) => {},
            SkyarchInstr::Stic(stic) => {
                let addr_reg = stic.dest();
                let src = stic.src();

                let addr = cpu.registers().read_gpr(addr_reg);

                let width = stic.width();

                let val = cpu.registers().read_gpr(src);

                if (addr & width.amask()) != LeU32::zero() {
                    return Err(SkyarchException::BusError)
                }

                cpu.flags_mut().set_undefined();

                if Some(width) != cpu.def_status_mut().il_width.take() {
                    cpu.flags_mut().set_z(true);
                } else if addr != cpu.def_state().il_addr {
                    cpu.flags_mut().set_z(true);
                } else {
                    cpu.flags_mut().set_z(false);
                    match width {
                        crate::instr::MemWidth::Byte => cpu.write(addr, val.truncate::<u8>())?,
                        crate::instr::MemWidth::Half => cpu.write(addr, val.truncate::<u16>())?,
                        crate::instr::MemWidth::Word => cpu.write(addr, val)?,
                        crate::instr::MemWidth::Double => panic!("64-bit write not allowed (use sticw)"),
                    }
                }
            },
            
            SkyarchInstr::Sticw(stic) => {
                let addr_reg = stic.dest();
                let src = stic.src();

                let addr = cpu.registers().read_gpr(addr_reg);

                let width = MemWidth::Double;

                let val = cpu.registers().read_gpr(src);
                let valhi = cpu.registers().read_gpr(stic.src2());

                if (addr & width.amask()) != LeU32::zero() {
                    return Err(SkyarchException::BusError)
                }

                cpu.flags_mut().set_undefined();

                if Some(width) != cpu.def_status_mut().il_width.take() {
                    cpu.flags_mut().set_z(true);
                } else if addr != cpu.def_state().il_addr {
                    cpu.flags_mut().set_z(true);
                } else {
                    cpu.flags_mut().set_z(false);
                    let val = [val, valhi];

                    cpu.write(addr, val)?;
                }
            },
            SkyarchInstr::Ldil(ldil) => {
                let addr_reg = ldil.src();
                let dest = ldil.dest();

                let addr = cpu.registers().read_gpr(addr_reg);

                let width = ldil.width();

                if (addr & width.amask()) != LeU32::zero() {
                    return Err(SkyarchException::BusError)
                }


                let val = match width {
                    crate::instr::MemWidth::Byte => cpu.read::<LeU8>(addr)?.extend(),
                    crate::instr::MemWidth::Half => cpu.read::<LeU16>(addr)?.extend(),
                    crate::instr::MemWidth::Word => cpu.read::<LeU32>(addr)?,
                    crate::instr::MemWidth::Double => panic!("64-bit read not allowed (use ldilw)"),
                };

                let status = cpu.def_status_mut();
                status.il_addr = addr;
                status.il_width = Some(width);

                cpu.registers_mut().write_gpr(dest, val);
            },
            SkyarchInstr::Ldilw(ldil) => {
                let addr_reg = ldil.src();
                let dest = ldil.dest();

                let addr = cpu.registers().read_gpr(addr_reg);

                let width = MemWidth::Double;

                if (addr & width.amask()) != LeU32::zero() {
                    return Err(SkyarchException::BusError)
                }


                let [val, valhi]: [LeU32; 2] = cpu.read(addr)?;

                let status = cpu.def_status_mut();
                status.il_addr = addr;
                status.il_width = Some(width);

                cpu.registers_mut().write_gpr(dest, val);
                cpu.registers_mut().write_gpr(ldil.dest2(), valhi);
            },
        }

        Ok(None)
    }

    fn handle_except<M: emu_lib::memory::MemorySupplier<Self::PAddr, Data = Self::CacheLine>>(cpu: &mut emu_lib::cpu::Cpu<Self, M>, except: Self::Exception) -> Result<Self::VAddr, Option<Self::Exception>> {
        let ictl = cpu.registers().intr().intctl;

        let mut real_except = except;

        let ip = cpu.current_ip();

        let intr = cpu.registers_mut().intr_mut();

        if ictl.abort() {
            return Err(None)
        } else if ictl.mask() < LeU8::from_ne(1) {
            real_except = SkyarchException::Abort;
            intr.intctl = Intctl::new().with_abort(true);
        } else {
            let iret = Intret::new().with_addr(AlignedAddr::new(ip)).with_retmask(ictl.mask());
            
            intr.intctl = Intctl::new().with_abort(true);
            intr.intret[0] = iret;
        }

        let slot = LeU32::from_ne((real_except as u32) << 3);

        let addr = intr.inttab.addr().into_inner() + slot;

        let imap = cpu.read::<IntSlot>(addr).map_err(|e| Some(e))?;

        if !imap.is_valid() || !imap.present() {
            return Err(Some(SkyarchException::Consistency))
        }

        let addr = imap.addr().into_inner();
        Ok(addr)
    }
}