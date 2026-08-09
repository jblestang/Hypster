//! COM1 serial output for QEMU `-serial stdio` integration tests.

const COM1: u16 = 0x3F8;

/// Writes a UTF-8 string to COM1, ignoring non-ASCII bytes.
pub fn write_str(message: &str) {
    for byte in message.bytes() {
        write_byte(byte);
    }
}

fn write_byte(byte: u8) {
    wait_transmit_empty();
    // SAFETY: COM1 data register write for debug output under QEMU/OVMF.
    unsafe {
        core::arch::asm!("out dx, al", in("dx") COM1, in("al") byte, options(nomem, nostack));
    }
}

fn wait_transmit_empty() {
    loop {
        let lsr: u8;
        // SAFETY: COM1 line status register read.
        unsafe {
            core::arch::asm!(
                "in al, dx",
                in("dx") COM1 + 5,
                out("al") lsr,
                options(nomem, nostack)
            );
        }
        if lsr & 0x20 != 0 {
            break;
        }
    }
}
