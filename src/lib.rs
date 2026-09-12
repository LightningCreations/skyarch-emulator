#![feature(
    min_adt_const_params,
    macro_metavar_expr_concat,
    const_trait_impl,
    const_try,
    const_ops,
    const_cmp,
    funnel_shifts
)]

pub mod cpu;

pub mod except;
pub mod intr;
pub mod regs;

pub mod instr;

#[cfg(test)]
mod test;
