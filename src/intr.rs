
use emu_lib::datatypes::*;

use emu_lib::{bitfield, bitfield::AlignedAddr};

bitfield!{
    #[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Pod, Zeroable)]
    pub struct Intctl : LeU32 {
        pub mask @ 0..2: LeU8,
        pub abort @ 31: bool,
    }
}

bitfield! {
    #[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Pod, Zeroable)]
    pub struct Intret : LeU32 {
        pub retmask @ 0..2: LeU8,
        pub addr @ 2..32: AlignedAddr<LeU32, 2>,
    }
}

bitfield! {
    #[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Pod, Zeroable)]
    pub struct Inttab : LeU32 {
        pub addr @ 3..32: AlignedAddr<LeU32, 3>,
    }
}