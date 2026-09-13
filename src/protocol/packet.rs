pub const REPORT_LENGTH: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Command {
    Temperature = 0x01,
    Frequency = 0x02,
    Usage = 0x03,
    Show = 0x04,
}

pub fn calculate_checksum(
    header1: u8,
    header2: u8,
    len: u8,
    cmd: u8,
    val_hi: u8,
    val_lo: u8,
) -> u8 {
    ((header1 as u16 + header2 as u16 + len as u16 + cmd as u16 + val_hi as u16 + val_lo as u16)
        & 0xFF) as u8
}

/// Builds a 64-byte frame according to the ID-COOLING FX LCD protocol:
/// [0x55, 0xBB, 0x02, cmd, val_hi, val_lo, cksum, 0x00 x 57]
pub fn build_frame(command: Command, value: u16) -> [u8; REPORT_LENGTH] {
    let mut frame = [0u8; REPORT_LENGTH];
    let h1 = 0x55;
    let h2 = 0xBB;
    let len = 0x02;
    let cmd = command as u8;
    let val_hi = ((value >> 8) & 0xFF) as u8;
    let val_lo = (value & 0xFF) as u8;
    let cksum = calculate_checksum(h1, h2, len, cmd, val_hi, val_lo);

    frame[0] = h1;
    frame[1] = h2;
    frame[2] = len;
    frame[3] = cmd;
    frame[4] = val_hi;
    frame[5] = val_lo;
    frame[6] = cksum;

    frame
}

/// Builds a 65-byte HID report buffer with a prepended Report ID (0x00).
/// HIDAPI on Windows, Linux (hidraw), and macOS expects the Report ID as the first byte.
/// Structure: [0x00, 0x55, 0xBB, 0x02, cmd, val_hi, val_lo, cksum, 0x00 x 57]
pub fn build_hid_report(command: Command, value: u16) -> [u8; REPORT_LENGTH + 1] {
    let mut report = [0u8; REPORT_LENGTH + 1];
    let frame = build_frame(command, value);
    report[0] = 0x00; // Unnumbered Report ID required by HIDAPI
    report[1..].copy_from_slice(&frame);
    report
}

/// Backwards-compatible alias for `build_hid_report`.
#[allow(dead_code)]
#[inline]
pub fn build_windows_report(command: Command, value: u16) -> [u8; REPORT_LENGTH + 1] {
    build_hid_report(command, value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temperature_frame() {
        // Temperature = 45 C (0x002D)
        let frame = build_frame(Command::Temperature, 45);
        assert_eq!(frame[0], 0x55);
        assert_eq!(frame[1], 0xBB);
        assert_eq!(frame[2], 0x02);
        assert_eq!(frame[3], 0x01);
        assert_eq!(frame[4], 0x00);
        assert_eq!(frame[5], 0x2D);
        // cksum = (0x55 + 0xBB + 0x02 + 0x01 + 0x00 + 0x2D) & 0xFF = 0x140 & 0xFF = 0x40
        let expected_cksum = (0x55u16 + 0xBBu16 + 0x02u16 + 0x01u16 + 0x2Du16) as u8;
        assert_eq!(frame[6], expected_cksum);
        assert_eq!(&frame[7..], &[0u8; 57]);
    }

    #[test]
    fn test_frequency_frame() {
        // Frequency = 4200 MHz (0x1068)
        let frame = build_frame(Command::Frequency, 4200);
        assert_eq!(frame[3], 0x02);
        assert_eq!(frame[4], 0x10);
        assert_eq!(frame[5], 0x68);
    }

    #[test]
    fn test_show_frames() {
        let frame_on = build_frame(Command::Show, 1);
        assert_eq!(frame_on[3], 0x04);
        assert_eq!(frame_on[4], 0x00);
        assert_eq!(frame_on[5], 0x01);

        let frame_off = build_frame(Command::Show, 0);
        assert_eq!(frame_off[3], 0x04);
        assert_eq!(frame_off[4], 0x00);
        assert_eq!(frame_off[5], 0x00);
    }

    #[test]
    fn test_windows_report_prepended_id() {
        let report = build_windows_report(Command::Temperature, 50);
        assert_eq!(report.len(), 65);
        assert_eq!(report[0], 0x00);
        assert_eq!(report[1], 0x55);
        assert_eq!(report[2], 0xBB);
    }

    #[test]
    fn test_build_hid_report() {
        let report = build_hid_report(Command::Frequency, 3600);
        assert_eq!(report.len(), 65);
        assert_eq!(report[0], 0x00);
        assert_eq!(report[1], 0x55);
        assert_eq!(report[2], 0xBB);
        assert_eq!(report[4], 0x02);
    }
}
