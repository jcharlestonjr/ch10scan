// ch10write: writes IRIG 106 Chapter 10 files.
// Step 1: a TMATS-only file. Milestone 3: one packet per AI confidence reading.
//
// Session plans: SESSION-ch10write.md, then SESSION-milestone3.md
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
const TYPE_USER_DEFINED: u8 = 0x00; // Computer Generated Format 0 (user-defined)
const READING_BYTES: u32 = 12; // u32 index + f32 entropy + f32 top_prob

// Minimal TMATS. The G\COM line documents the user-defined F0 record layout.
const TMATS: &str = "G\\PN:AI-TELEMETRY;\n\
G\\106:15;\n\
G\\COM:ch 0 F0 = AI confidence: u32 index, f32 entropy (nats), f32 top_prob;\n\
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

/// One row of confidence.csv (the token text stays in the CSV).
struct Reading {
    index: u32,
    t_ns: u64,
    entropy: f32,
    top_prob: f32,
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

// ======================= Milestone 3 =======================
// Session plan: SESSION-milestone3.md

/// Read confidence.csv from the 4060 run. Pre-filled: the csv crate handles
/// tokens that contain commas, quotes or newlines, which a split(',') can't.
fn read_readings(path: &str) -> Vec<Reading> {
    let mut reader = csv::Reader::from_path(path).expect("could not open CSV");
    let mut readings = Vec::new();
    for record in reader.records() {
        let row = record.expect("bad CSV row");
        // columns: index, timestamp_ns, token, entropy, top_prob
        readings.push(Reading {
            index: row[0].parse().expect("bad index"),
            t_ns: row[1].parse().expect("bad timestamp_ns"),
            entropy: row[3].parse().expect("bad entropy"),
            top_prob: row[4].parse().expect("bad top_prob"),
        });
    }
    readings
}

/// Turn a nanosecond timestamp into relative time counter ticks.
/// The RTC runs at 10 MHz, so one tick = 100 ns. t0_ns is the moment that becomes RTC 0.
fn rtc_from_ns(t_ns: u64, t0_ns: u64) -> u64 {
    // YOUR PART 3 (5 min, warm-up):
    // 1. how many ns since t0?
    // 2. turn ns into 100 ns ticks (integer division drops the leftover, that's fine)
    //
    // Done when: cargo test --bin ch10write rtc_
    todo!("rtc_from_ns")
}

/// One reading -> one Computer Generated Format 0 (user-defined) packet on channel 0.
/// Body: CSDW (4 bytes, all zero for F0), then u32 index, f32 entropy, f32 top_prob.
fn build_confidence_packet(r: &Reading, seq: u8, rtc: u64) -> Vec<u8> {
    // YOUR PART 4: same shape as build_tmats_packet, different payload.
    // 1. csdw: u32 = 0
    // 2. data_len = 4 (the CSDW) + READING_BYTES
    // 3. packet_len = 24 + data_len, rounded up to a multiple of 4 (works out to 40)
    // 4. a Header: channel 0, flags 0, version DATA_TYPE_VERSION,
    //    data_type TYPE_USER_DEFINED, plus the seq and rtc you were given
    // 5. extend_from_slice in this order: header, csdw, r.index, r.entropy, r.top_prob
    //    (all with .to_le_bytes(); f32 has it too)
    // 6. resize to packet_len and return it
    //
    // Done when: cargo test --bin ch10write confidence_packet_layout
    todo!("build_confidence_packet")
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: ch10write <confidence.csv> <out.c10>");
        std::process::exit(1);
    }
    let readings = read_readings(&args[1]);
    let t0 = readings.first().expect("CSV has no readings").t_ns;

    let mut file = build_tmats_packet(TMATS, 0, 0);

    // YOUR PART 5: append one confidence packet per reading.
    // 1. let mut seq: u8 = 1;
    //    Sequence numbers count per channel, and channel 0 already used 0 for TMATS.
    // 2. for r in &readings {
    //        rtc = rtc_from_ns(r.t_ns, t0)
    //        append build_confidence_packet(r, seq, rtc) to file
    //        seq = seq.wrapping_add(1)   (rolls 255 -> 0, as the standard expects)
    //    }
    //
    // Done when: ch10scan shows 58 F0 packets and "OK: all 2516 bytes accounted for"

    fs::write(&args[2], &file).expect("could not write file");
    eprintln!("wrote {} bytes to {}", file.len(), args[2]);
}

// ------------------------------------------------------------------
// DONE CHECKS: `cargo test --bin ch10write`. Step 1 = first five, milestone 3 = all nine.
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

    // ---------------- Milestone 3: confidence packets ----------------

    #[test]
    fn reads_sample_csv() {
        let readings = read_readings("confidence_sample.csv");
        assert_eq!(readings.len(), 58);
        assert_eq!(readings[6].index, 6);
        assert!((readings[6].entropy - 3.811_142).abs() < 1e-5);
    }

    #[test]
    fn rtc_starts_at_zero() {
        assert_eq!(rtc_from_ns(1795155421998010048, 1795155421998010048), 0);
    }

    #[test]
    fn rtc_converts_ns_to_10mhz_ticks() {
        // First two tokens in confidence_sample.csv are 59,518,610 ns apart.
        // At 10 MHz that's 595,186 ticks (the leftover 10 ns gets dropped).
        assert_eq!(rtc_from_ns(1795155422057528658, 1795155421998010048), 595_186);
    }

    #[test]
    fn confidence_packet_layout() {
        let r = Reading { index: 6, t_ns: 0, entropy: 3.811_142, top_prob: 0.136_583 };
        let p = build_confidence_packet(&r, 7, 595_186);

        assert_eq!(p.len(), 40); // 24 header + 4 CSDW + 12 reading, already aligned
        assert_eq!(u16::from_le_bytes([p[2], p[3]]), 0); // channel 0
        assert_eq!(u32::from_le_bytes([p[4], p[5], p[6], p[7]]), 40); // packet length
        assert_eq!(u32::from_le_bytes([p[8], p[9], p[10], p[11]]), 16); // data length
        assert_eq!(p[13], 7); // sequence number
        assert_eq!(p[15], TYPE_USER_DEFINED);
        let rtc = u64::from_le_bytes([p[16], p[17], p[18], p[19], p[20], p[21], 0, 0]);
        assert_eq!(rtc, 595_186);
        assert_eq!(header_checksum(&p[..24]), u16::from_le_bytes([p[22], p[23]]));

        assert_eq!(u32::from_le_bytes([p[24], p[25], p[26], p[27]]), 0); // CSDW
        assert_eq!(u32::from_le_bytes([p[28], p[29], p[30], p[31]]), 6); // index
        assert_eq!(f32::from_le_bytes([p[32], p[33], p[34], p[35]]), r.entropy);
        assert_eq!(f32::from_le_bytes([p[36], p[37], p[38], p[39]]), r.top_prob);
    }
}
