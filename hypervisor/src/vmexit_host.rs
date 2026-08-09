//! VM-exit host entry trampoline.

core::arch::global_asm!(
    ".global vmexit_entry",
    "vmexit_entry:",
    "call vmexit_dispatch",
);
