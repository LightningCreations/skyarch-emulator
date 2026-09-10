use std::{rc::Rc, sync::Arc};

use bytemuck::Pod;
use emu_lib::{cpu::{Cpu, StopReason}, datatypes::{LeInt, LeU8, LeU32}, io::TestPort, memory::MemoryControllerNonSync};

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

    let port = Arc::new(TestPort::new(LeU8::from_ne(0xFF), expected));

    cpu.attach_port_dyn(Arc::<TestPort<_, _>>::clone(&port));

    cpu.reset();

    let reason = cpu.run();

    assert_eq!(reason, StopReason::Halted);
}

fn run_halt_test(rom: &[LeU32]) {
    let rom = bytemuck::cast_slice(rom);

    let mut cpu = Cpu::new(Skyarch::new(), Rc::new(MemoryControllerNonSync::new(256)), rom, LeU32::zero());

    cpu.reset();

    let reason = cpu.run();

    assert_eq!(reason, StopReason::Halted);
}

fn run_reset_test(rom: &[LeU32]) {
    let rom = bytemuck::cast_slice(rom);

    let mut cpu = Cpu::new(Skyarch::new(), Rc::new(MemoryControllerNonSync::new(256)), rom, LeU32::zero());

    cpu.reset();

    let reason = cpu.run();

    assert_eq!(reason, StopReason::Reset);
}

#[test]
fn test_halt_works() {
    run_halt_test(&*exec_rom(&[LeInt::from_ne(0x00000040)]));
}

#[test]
fn test_reset_works() {
    // should immediately reset
    run_reset_test(&*exec_rom(&[LeInt::from_ne(0x00000000)]));
}

#[test]
#[should_panic]
fn test_output_works() {
    run_test(&*exec_rom(&[
        LeInt::from_ne(0x0A01E002),
        LeInt::from_ne(0x001FE015),
        LeInt::from_ne(0x00000040)
    ]), LeU32::mask())
}

#[test]
fn test_mov() {
    run_test(&*exec_rom(&[
        LeInt::from_ne(0x0A01E002),
        LeInt::from_ne(0x001FE015),
        LeInt::from_ne(0x00000040)
    ]), LeU32::zero())
}

#[test]
fn test_ldi() {
    run_test(&*exec_rom(&[
        LeInt::from_ne(0xDEAF0105),
        LeInt::from_ne(0x0A05E002),
        LeInt::from_ne(0x001FE015),
        LeInt::from_ne(0x00000040)
    ]), LeU32::from_ne(0xDEAF));
}

#[test]
fn test_ldi2() {
    run_test(&*exec_rom(&[
        LeInt::from_ne(0x13370105),
        LeInt::from_ne(0x0A05E002),
        LeInt::from_ne(0x001FE015),
        LeInt::from_ne(0x00000040)
    ]), LeU32::from_ne(0x1337));
}

#[test]
fn test_addi() {
    run_test(&*exec_rom(&[
        LeInt::from_ne(0xBEEF0105),
        LeInt::from_ne(0xDEAD8108),
        LeInt::from_ne(0x0A05E002),
        LeInt::from_ne(0x001FE015),
        LeInt::from_ne(0x00000040)
    ]), LeU32::from_ne(0xDEADBEEF));
}