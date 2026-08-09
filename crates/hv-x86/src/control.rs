//! VMX-related control register and feature-control bit constants.

/// CR4.VMXE — VMX operation enable bit.
pub const CR4_VMX_ENABLE: u64 = 1 << 13;

/// `IA32_FEATURE_CONTROL` lock bit.
pub const FEATURE_CONTROL_LOCKED: u64 = 1 << 0;
/// `IA32_FEATURE_CONTROL` VMX inside SMX bit.
pub const FEATURE_CONTROL_VMX_IN_SMX: u64 = 1 << 1;
/// `IA32_FEATURE_CONTROL` VMX outside SMX bit.
pub const FEATURE_CONTROL_VMX_OUTSIDE_SMX: u64 = 1 << 2;
