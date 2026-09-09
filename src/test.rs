use std::rc::Rc;

use bytemuck::Pod;
use emu_lib::{cpu::{Cpu, StopReason}, datatypes::{LeU8, LeU32}, io::TestPort, memory::MemoryControllerNonSync};

use crate::cpu::Skyarch;

const ROM_SIZE: usize = 65536/4;

const EXEC_ADDR: usize = (0xFF00)/4;

fn exec_rom(exec: &[LeU32]) -> Box<[LeU32; ROM_SIZE]> {
    make_rom(&[], exec)
}

fn make_rom(data: &[LeU32], exec: &[LeU32]) -> Box<[LeU32; ROM_SIZE]> {
    let mut rom = bytemuck::zeroed_box::<[LeU32; ROM_SIZE]>();

    let mut i = 0;

    while i < data.len() {
        rom[i] = data[i];
        i += 1;
    }

    let mut i = 0;

    while i < exec.len() {
        rom[i + EXEC_ADDR] = exec[i];
        i += 1;
    }

    rom
}

fn run_test(rom: &[LeU32], expected: LeU32) {
    let rom = bytemuck::cast_slice(rom);

    let mut cpu = Cpu::new(Skyarch::new(), Rc::new(MemoryControllerNonSync::new(256)), rom, LeU32::zero());

    cpu.attach_port(TestPort::new(LeU8::from_ne(0xFF), expected));

    let reason = cpu.run();

    assert_eq!(reason, StopReason::Halted);
}

fn run_reset_test(rom: &[LeU32]) {
    let rom = bytemuck::cast_slice(rom);

    let mut cpu = Cpu::new(Skyarch::new(), Rc::new(MemoryControllerNonSync::new(256)), rom, LeU32::zero());

    let reason = cpu.run();

    assert_eq!(reason, StopReason::Halted);
}