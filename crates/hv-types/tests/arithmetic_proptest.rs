#![allow(missing_docs)]
#![allow(clippy::expect_used)] // Unit tests use fixed valid PCI BDF literals.

use hv_types::arithmetic::{
    align_up_u64, checked_add_u64, checked_mul_u64, gib_to_bytes, is_aligned_u64,
    ranges_overlap_u64, ArithmeticError,
};
use hv_types::pci::PciBdf;
use proptest::prelude::*;

#[test]
fn checked_add_u64_ok() {
    assert_eq!(checked_add_u64(1, 2).ok(), Some(3));
}

#[test]
fn checked_add_u64_overflow() {
    assert_eq!(checked_add_u64(u64::MAX, 1), Err(ArithmeticError::Overflow));
}

#[test]
fn align_up_u64_works() {
    assert_eq!(align_up_u64(1, 4096).ok(), Some(4096));
    assert_eq!(align_up_u64(4096, 4096).ok(), Some(4096));
}

#[test]
fn gib_to_bytes_known_values() {
    assert_eq!(gib_to_bytes(1).ok(), Some(1024 * 1024 * 1024));
}

#[test]
fn ranges_overlap_detects_intersection() {
    assert!(ranges_overlap_u64(0, 10, 5, 10));
    assert!(!ranges_overlap_u64(0, 5, 5, 10));
}

#[test]
fn pci_bdf_parse_and_format() {
    let bdf = PciBdf::parse("0000:00:03.0").expect("valid bdf");
    assert_eq!(bdf.format(), *b"0000:00:03.0");
}

proptest! {
    #[test]
    fn align_up_is_monotonic(value in 0u64..(1 << 40), shift in 0u32..16) {
        let alignment = 1u64 << shift;
        let aligned = align_up_u64(value, alignment)?;
        prop_assert!(aligned >= value);
        prop_assert!(is_aligned_u64(aligned, alignment)?);
    }

    #[test]
    fn checked_mul_never_wraps_silently(a in 0u64..1024, b in 0u64..1024) {
        let expected = a.checked_mul(b);
        prop_assert_eq!(checked_mul_u64(a, b).ok(), expected);
    }
}
