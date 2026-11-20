// ch10write: writes IRIG 106 Chapter 10 files.
// Step 1: a file that holds only a TMATS packet.
//
// Session plan: SESSION-ch10write.md
//
// Primary header, 24 bytes, all little-endian (same layout parse_header reads):
//    0..2   sync 0xEB25           12  data type version
//    2..4   channel ID            13  sequence number
//    4..8   packet length         14  packet flags
//    8..12  data length           15  data type
//   16..22  relative time counter (6 bytes, 10 MHz ticks)
//   22..24  header checksum (16-bit sum of bytes 0..22 as LE words)
// After the header: CSDW (4 bytes), TMATS text, zero filler to a multiple of 4.
//
// data length   = CSDW + text            (no header, no filler)
// packet length = header + data + filler (the whole packet)

#![allow(unused_variables, dead_code)] // delete this line once both YOUR PARTs are done

use std::env;
use std::fs;

const SYNC: u16 = 0xEB25;
// A matched pair copied from ethernet.c10's TMATS packet (IRIG 106-15):
const DATA_TYPE_VERSION: u8 = 0x07; // header byte 12
const CH10VER: u32 = 0x0B; // low byte of the TMATS CSDW
const TYPE_TMATS: u8 = 0x01; // Computer Generated Format 1

// Minimal TMATS for now. Real channel definitions come with the data step.
const TMATS: &str = "G\\PN:AI-TELEMETRY;\n\
G\\106:15;\n\
G\\COM:ch10write step 1 - TMATS only;\n\
G\\DSI\\N:1;\n\
G\\DSI-1:AIREC;\n\
G\\DST-1:OTH;\n\
R-1\\ID:AIREC;\n\
R-1\\N:0;\n";

struct Header {
    channel_id: u16,
    packet_len: u32,
    data_len: u32,
    version: u8,
    seq: u8,
    flags: u8,
    data_type: u8,
    rtc: u64,
}

// Copied from main.rs: your checksum, unchanged.
fn header_checksum(h: &[u8]) -> u16 {
    let mut sum: u16 = 0;
    for i in (0..22).step_by(2) {
        let check = u16::from_le_bytes([h[i], h[i + 1]]);
        sum = sum.wrapping_add(check);
    }
    sum
}

/// Turn a Header into the 24 bytes that go on disk.
fn build_header(hdr: &Header) -> [u8; 24] {
    // YOUR PART 1: this is parse_header() from main.rs, run backwards.
    // 1. let mut h = [0u8; 24];
    // 2. bytes 0..2 = SYNC as little-endian
    //    (one field at a time: h[0..2].copy_from_slice(&SYNC.to_le_bytes());)
    // 3. bytes 2..4 = channel_id, 4..8 = packet_len, 8..12 = data_len
    // 4. byte 12 = version, 13 = seq, 14 = flags, 15 = data_type
    // 5. bytes 16..22 = the low 6 bytes of rtc: &hdr.rtc.to_le_bytes()[..6]
    // 6. LAST: bytes 22..24 = header_checksum(&h) as little-endian
    //    (the checksum covers bytes 0..22, so those must be filled first)
    // 7. return h
    //
    // Done when: cargo test --bin ch10write builds_known_header
    todo!("build_header")
}

/// Wrap TMATS text in a complete Computer Generated Format 1 packet.
fn build_tmats_packet(text: &str, seq: u8, rtc: u64) -> Vec<u8> {
    // YOUR PART 2:
    // 1. csdw: u32 = CH10VER  (bit 8 stays 0, which means ASCII TMATS)
    // 2. data_len = 4 (the CSDW) + text.len()
    // 3. packet_len = 24 + data_len, rounded UP to a multiple of 4
    //    (round-up trick: (n + 3) / 4 * 4)
    // 4. make a Header: channel 0, flags 0 (no secondary header, no data
    //    checksum), version DATA_TYPE_VERSION, data_type TYPE_TMATS
    // 5. let mut packet = Vec::new(); then extend_from_slice with, in order:
    //    the header bytes, csdw.to_le_bytes(), text.as_bytes()
    // 6. pad with zeros up to packet_len: packet.resize(packet_len as usize, 0);
    // 7. return packet
    //
    // Done when: cargo test --bin ch10write   (all 5 green)
    todo!("build_tmats_packet")
}

fn main() {
    let path = env::args().nth(1).expect("usage: ch10write <out.c10>");
    let packet = build_tmats_packet(TMATS, 0, 0);
    fs::write(&path, &packet).expect("could not write file");
    eprintln!("wrote {} bytes to {path}", packet.len());
}

// ------------------------------------------------------------------
// DONE CHECKS: run `cargo test --bin ch10write`. All five green = done.
// ------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    // Packet 1 header from discrete.c10, the same bytes ch10scan's tests use.
    // If your builder can reproduce a real recorder's header byte for byte,
    // it's right.
    const DISCRETE_TMATS_HEADER: [u8; 24] = [
        0x25, 0xeb, 0x00, 0x00, 0x00, 0x6e, 0x00, 0x00,
        0xb8, 0x43, 0x00, 0x00, 0x05, 0x00, 0x00, 0x01,
        0x25, 0x0a, 0xa3, 0xb8, 0x06, 0x00, 0xb0, 0x60,
    ];

    #[test]
    fn builds_known_header() {
        let hdr = Header {
            channel_id: 0,
            packet_len: 28160,
            data_len: 17336,
            version: 0x05,
            seq: 0,
            flags: 0,
            data_type: 0x01,
            rtc: 28867496485,
        };
        assert_eq!(build_header(&hdr), DISCRETE_TMATS_HEADER);
    }

    #[test]
    fn pads_to_32_bit_boundary() {
        // 24 header + 4 CSDW + 3 text = 31 bytes, so 1 byte of zero filler
        let packet = build_tmats_packet("ODD", 0, 0);
        assert_eq!(packet.len(), 32);
        assert_eq!(packet[31], 0);
    }

    #[test]
    fn length_fields_match_packet() {
        let packet = build_tmats_packet(TMATS, 0, 0);
        let packet_len = u32::from_le_bytes([packet[4], packet[5], packet[6], packet[7]]);
        let data_len = u32::from_le_bytes([packet[8], packet[9], packet[10], packet[11]]);
        assert_eq!(packet_len as usize, packet.len());
        assert_eq!(data_len as usize, 4 + TMATS.len());
    }

    #[test]
    fn header_checksum_is_valid() {
        let packet = build_tmats_packet(TMATS, 0, 0);
        let stored = u16::from_le_bytes([packet[22], packet[23]]);
        assert_eq!(header_checksum(&packet[..24]), stored);
    }

    #[test]
    fn tmats_text_round_trips() {
        let packet = build_tmats_packet(TMATS, 0, 0);
        let csdw = u32::from_le_bytes([packet[24], packet[25], packet[26], packet[27]]);
        assert_eq!(csdw, CH10VER); // bit 8 = 0 means ASCII TMATS
        assert_eq!(&packet[28..28 + TMATS.len()], TMATS.as_bytes());
    }
}
