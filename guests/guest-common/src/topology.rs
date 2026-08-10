//! Deterministic IPC layout for the `qemu-validation` topology.
//!
//! These constants match [`configs/qemu.yaml`](../../configs/qemu.yaml) after platform
//! resolution. Guests use them until boot-info-driven discovery is wired at launch.

/// Total shared bytes for one IPC ring in the validated topology.
pub const IPC_SHARED_BYTES: usize = 524_328;
/// IPC ring slot count from validated configuration.
pub const IPC_SLOT_COUNT: u32 = 256;
/// IPC ring slot payload size from validated configuration.
pub const IPC_SLOT_SIZE: u32 = 2048;

/// Guest-physical base of `in_to_mid` in the IN partition.
pub const IN_IN_TO_MID_GPA: u64 = 0x4000_0000;
/// Guest-physical base of `in_to_mid` in the MID partition.
pub const MID_IN_TO_MID_GPA: u64 = 0x8000_0000;
/// Guest-physical base of `mid_to_out` in the MID partition.
pub const MID_MID_TO_OUT_GPA: u64 = 0x8008_0148;

/// Channel names for the validated topology.
pub mod names {
    /// Producer channel from IN to MID.
    pub const IN_TO_MID: &str = "in_to_mid";
    /// Producer channel from MID to OUT.
    pub const MID_TO_OUT: &str = "mid_to_out";
}
