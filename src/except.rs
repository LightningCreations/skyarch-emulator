use emu_lib::memory::BusError;





#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
#[repr(u8)]
pub enum SkyarchException {
    Abort = 0,
    BusError = 1,
    Undefined = 2,
    UnalignedBranch = 3,
    Consistency = 4,
    Breakpoint = 5,
    PriorityIrq = 7,
    Coprocessor0 = 8,
    Coprocessor1 = 9,
    Coprocessor2 = 10,
    Coprocessor3 = 11,
    Coprocessor4 = 12,
    Coprocessor5 = 13,
    Coprocessor6 = 14,
    Coprocessor7 = 15,
}

impl From<BusError> for SkyarchException {
    fn from(_: BusError) -> Self {
        Self::BusError
    }
}