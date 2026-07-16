//! Minimal ANSI escape-sequence handling.
//!
//! `strip_ansi` removes:
//! * CSI sequences: `ESC [` followed by parameter / intermediate bytes and a
//!   final byte in `0x40..=0x7E` (e.g. `ESC[31m`, `ESC[2J`, `ESC[H`).
//! * OSC sequences: `ESC ]` terminated by `BEL` (`0x07`) or `ST` (`ESC \`).
//! * Other two-byte ESC sequences: `ESC` followed by anything else.
//!
//! This is intentionally not a full VT/ANSI state machine. A future upgrade
//! to `vte` is possible without changing any current call sites.

/// Returns `input` with all ANSI escape sequences removed.
pub fn strip_ansi(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        let b = input[i];
        if b != 0x1b {
            out.push(b);
            i += 1;
            continue;
        }
        // ESC sequence — peek at the introducer byte.
        if i + 1 >= input.len() {
            // Trailing lone ESC at end of stream — preserve it.
            out.push(b);
            i += 1;
            continue;
        }
        match input[i + 1] {
            b'[' => {
                // CSI: skip parameter bytes (0x30..=0x3F) and intermediate
                // bytes (0x20..=0x2F), then the final byte (0x40..=0x7E).
                i += 2;
                while i < input.len() {
                    let c = input[i];
                    if (0x40..=0x7E).contains(&c) {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
            }
            b']' => {
                // OSC: terminated by BEL (0x07) or ST (ESC \).
                i += 2;
                while i < input.len() {
                    if input[i] == 0x07 {
                        i += 1;
                        break;
                    }
                    if input[i] == 0x1b && i + 1 < input.len() && input[i + 1] == b'\\' {
                        i += 2;
                        break;
                    }
                    i += 1;
                }
            }
            _ => {
                // Two-byte ESC sequence: drop both bytes.
                i += 2;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passthrough_plain_text() {
        assert_eq!(strip_ansi(b"hello"), b"hello".to_vec());
    }

    #[test]
    fn strips_csi_sgr_color() {
        let input = b"\x1b[31mhi\x1b[0m";
        assert_eq!(strip_ansi(input), b"hi".to_vec());
    }

    #[test]
    fn strips_csi_cursor_move() {
        let input = b"\x1b[2J\x1b[Hhello";
        assert_eq!(strip_ansi(input), b"hello".to_vec());
    }

    #[test]
    fn strips_osc_bel_terminated() {
        let input = b"\x1b]0;title\x07after";
        assert_eq!(strip_ansi(input), b"after".to_vec());
    }

    #[test]
    fn strips_osc_st_terminated() {
        let input = b"\x1b]0;title\x1b\\after";
        assert_eq!(strip_ansi(input), b"after".to_vec());
    }

    #[test]
    fn handles_trailing_esc() {
        let input = b"abc\x1b";
        assert_eq!(strip_ansi(input), b"abc\x1b".to_vec());
    }

    #[test]
    fn strips_two_byte_esc() {
        // ESC c is "Reset Device" — out of scope, treat as two-byte and drop.
        let input = b"\x1bcreset\x1b";
        assert_eq!(strip_ansi(input), b"reset\x1b".to_vec());
    }
}
