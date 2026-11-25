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
use std::io::Write;
use std::net::UdpSocket;
use std::time::Duration;

const SYNC: u16 = 0xEB25;
// A matched pair copied from ethernet.c10's TMATS packet (IRIG 106-15):
const DATA_TYPE_VERSION: u8 = 0x07; // header byte 12
const CH10VER: u32 = 0x0B; // low byte of the TMATS CSDW
const TYPE_TMATS: u8 = 0x01; // Computer Generated Format 1
const TYPE_USER_DEFINED: u8 = 0x00; // Computer Generated Format 0 (user-defined)
const READING_BYTES: u32 = 12; // u32 index + f32 entropy + f32 top_prob
const TYPE_TIME: u8 = 0x11; // Time Data Format 1
const TIME_CHANNEL: u16 = 1; // time gets its own channel, like the sample files
// Time CSDW: FMT (bits 7-4) = 0x3 internal real-time clock, SRC (bits 3-0) = 0x0 internal.
// Same FMT/SRC as ethernet.c10. Bit 8 = leap year, bit 9 = 0 means day-of-year format.
const TIME_CSDW_RTC_INTERNAL: u32 = 0x30;
const DATAGRAM_BYTES: usize = 20; // u32 index + u64 t_ns + f32 entropy + f32 top_prob

// Minimal TMATS. The G\COM line documents the user-defined F0 record layout.
const TMATS: &str = "G\\PN:AI-TELEMETRY;\n\
G\\106:15;\n\
G\\COM:ch 0 F0 = AI confidence (u32 index, f32 entropy nats, f32 top_prob), ch 1 = Time F1 from PC clock;\n\
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

/// A UTC time broken into the pieces IRIG time needs.
struct UtcParts {
    year: u32,
    doy: u32, // day of year, Jan 1 = 1
    hour: u32,
    min: u32,
    sec: u32,
    ms: u32,
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
    let mut h = [0u8; 24];
    // 2. bytes 0..2 = SYNC as little-endian
    //    (one field at a time: h[0..2].copy_from_slice(&SYNC.to_le_bytes());)
    h[0..2].copy_from_slice(&SYNC.to_le_bytes());
    // 3. bytes 2..4 = channel_id, 4..8 = packet_len, 8..12 = data_len
    h[2..4].copy_from_slice(&hdr.channel_id.to_le_bytes());
    h[4..8].copy_from_slice(&hdr.packet_len.to_le_bytes());
    h[8..12].copy_from_slice(&hdr.data_len.to_le_bytes());
    // 4. byte 12 = version, 13 = seq, 14 = flags, 15 = data_type
    h[12] = hdr.version;
    h[13] = hdr.seq;
    h[14] = hdr.flags;
    h[15] = hdr.data_type;
    // 5. bytes 16..22 = the low 6 bytes of rtc: &hdr.rtc.to_le_bytes()[..6]
    h[16..22].copy_from_slice(&hdr.rtc.to_le_bytes()[..6]);
    // 6. LAST: bytes 22..24 = header_checksum(&h) as little-endian
    //    (the checksum covers bytes 0..22, so those must be filled first)
    let checksum = header_checksum(&h);
    h[22..24].copy_from_slice(&checksum.to_le_bytes());
    // 7. return h
    return h;
    // Done when: cargo test --bin ch10write builds_known_header
    //todo!("build_header")
}

/// Wrap TMATS text in a complete Computer Generated Format 1 packet.
fn build_tmats_packet(text: &str, seq: u8, rtc: u64) -> Vec<u8> {
    // YOUR PART 2:
    // 1. csdw: u32 = CH10VER  (bit 8 stays 0, which means ASCII TMATS)
    let csdw = CH10VER;
    // 2. data_len = 4 (the CSDW) + text.len()
    let data_len = 4 + text.len() as u32;
    // 3. packet_len = 24 + data_len, rounded UP to a multiple of 4
    //    (round-up trick: (n + 3) / 4 * 4)
    let packet_len = 24 + data_len;
    let packet_len = (packet_len + 3) / 4 * 4;
    // 4. make a Header: channel 0, flags 0 (no secondary header, no data
    //    checksum), version DATA_TYPE_VERSION, data_type TYPE_TMATS
    let hdr = Header {
        channel_id: 0,
        packet_len,
        data_len,
        version: DATA_TYPE_VERSION,
        seq,
        flags: 0,
        data_type: TYPE_TMATS,
        rtc,
    };
    // 5. let mut packet = Vec::new(); then extend_from_slice with, in order:
    //    the header bytes, csdw.to_le_bytes(), text.as_bytes()
    let mut packet = Vec::new();
    packet.extend_from_slice(&build_header(&hdr));
    packet.extend_from_slice(&csdw.to_le_bytes());
    packet.extend_from_slice(text.as_bytes());
    // 6. pad with zeros up to packet_len: packet.resize(packet_len as usize, 0);
    packet.resize(packet_len as usize, 0);
    // 7. return packet
    return packet;
    //
    // Done when: cargo test --bin ch10write   (all 5 green)
    //todo!("build_tmats_packet")
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
    let t = t_ns - t0_ns;
    let t_mhz: u64 = t / 100;
    // Done when: cargo test --bin ch10write rtc_
    //todo!("rtc_from_ns")
    return t_mhz;
}

/// One reading -> one Computer Generated Format 0 (user-defined) packet on channel 0.
/// Body: CSDW (4 bytes, all zero for F0), then u32 index, f32 entropy, f32 top_prob.
fn build_confidence_packet(r: &Reading, seq: u8, rtc: u64) -> Vec<u8> {
    // YOUR PART 4: same shape as build_tmats_packet, different payload.
    // 1. csdw: u32 = 0
    let csdw: u32 = 0;
    // 2. data_len = 4 (the CSDW) + READING_BYTES
    let data_len: u32 = 4 + READING_BYTES;
    // 3. packet_len = 24 + data_len, rounded up to a multiple of 4 (works out to 40)
    let packet_len: u32 = 24 + data_len;
    let packet_len: u32 = (packet_len + 3) / 4 * 4;
    // 4. a Header: channel 0, flags 0, version DATA_TYPE_VERSION,
    //    data_type TYPE_USER_DEFINED, plus the seq and rtc you were given
    // 5. extend_from_slice in this order: header, csdw, r.index, r.entropy, r.top_prob
    //    (all with .to_le_bytes(); f32 has it too)
    // 6. resize to packet_len and return it
    let hdr = Header {
        channel_id: 0,
        packet_len,
        data_len,
        version: DATA_TYPE_VERSION,
        seq,
        flags: 0,
        data_type: TYPE_USER_DEFINED,
        rtc,
    };
    //
    let mut packet = Vec::new();
    packet.extend_from_slice(&build_header(&hdr));
    packet.extend_from_slice(&csdw.to_le_bytes());
    packet.extend_from_slice(&r.index.to_le_bytes());
    packet.extend_from_slice(&r.entropy.to_le_bytes());
    packet.extend_from_slice(&r.top_prob.to_le_bytes());
    packet.resize(packet_len as usize, 0);
    return packet;
    // Done when: cargo test --bin ch10write confidence_packet_layout
    //todo!("build_confidence_packet")
}

// ======================= Milestone 4 =======================
// Session plan: SESSION-milestone4.md

fn is_leap(year: u32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Unix nanoseconds -> UTC pieces. Pre-filled: this is calendar math, not Ch10.
fn utc_parts(t_ns: u64) -> UtcParts {
    let secs = t_ns / 1_000_000_000;
    let ms = ((t_ns / 1_000_000) % 1000) as u32;
    let mut days = secs / 86_400; // whole days since 1970-01-01
    let rem = secs % 86_400; // seconds into today
    let mut year: u32 = 1970;
    loop {
        let year_len = if is_leap(year) { 366 } else { 365 };
        if days < year_len {
            break;
        }
        days -= year_len;
        year += 1;
    }
    UtcParts {
        year,
        doy: days as u32 + 1,
        hour: (rem / 3600) as u32,
        min: (rem % 3600 / 60) as u32,
        sec: (rem % 60) as u32,
        ms,
    }
}

/// Pack UTC pieces into the three BCD words of IRIG day-of-year time.
fn bcd_doy_words(p: &UtcParts) -> [u16; 3] {
    // YOUR PART 6: BCD = one decimal digit per 4-bit field.
    // For 58 seconds: units digit = 58 % 10 = 8, tens digit = 58 / 10 = 5.
    //
    // word 0: bits 3-0  tens of ms      (ms / 10 % 10)
    //         bits 7-4  hundreds of ms  (ms / 100)
    //         bits 11-8 units of seconds
    //         bits 14-12 tens of seconds
    // word 1: bits 3-0  units of minutes,  bits 6-4   tens of minutes
    //         bits 11-8 units of hours,    bits 13-12 tens of hours
    // word 2: bits 3-0  units of day,      bits 7-4   tens of day
    //         bits 9-8  hundreds of day
    //
    let w0: u32 = (p.ms / 10 % 10) | ((p.ms / 100) << 4) | ((p.sec % 10) << 8) | ((p.sec / 10) << 12);
    let w1: u32 = (p.min % 10) | ((p.min / 10) << 4) | ((p.hour % 10) << 8) | ((p.hour / 10) << 12);
    let w2: u32 = (p.doy % 10) | (((p.doy / 10) % 10) << 4) | ((p.doy / 100) << 8);
    [w0 as u16, w1 as u16, w2 as u16]
}

/// One Time Data Format 1 packet: "at this RTC, the clock read this time."
fn build_time_packet(t_ns: u64, seq: u8, rtc: u64) -> Vec<u8> {
    // YOUR PART 7: same shape as your other packets.
    // 1. parts = utc_parts(t_ns)
    let parts = utc_parts(t_ns);
    // 2. let mut csdw: u32 = TIME_CSDW_RTC_INTERNAL;
    let mut csdw: u32 = TIME_CSDW_RTC_INTERNAL;
    if is_leap(parts.year) {
        csdw |= 1 << 8;
    }
    let words = bcd_doy_words(&parts);
    let data_len: u32 = 4 + 6; // 4 (CSDW) + 6 (three u16 words)
    let packet_len: u32 = 24 + data_len;
    let packet_len: u32 = (packet_len + 3) / 4 * 4; // round up to multiple of 4
    let hdr = Header {
        channel_id: TIME_CHANNEL,
        packet_len,
        data_len,
        version: DATA_TYPE_VERSION,
        seq,
        flags: 0,
        data_type: TYPE_TIME,
        rtc,
    };
    let mut packet = Vec::new();
    packet.extend_from_slice(&build_header(&hdr));
    packet.extend_from_slice(&csdw.to_le_bytes());
    for word in &words {
        packet.extend_from_slice(&word.to_le_bytes());
    }
    packet.resize(packet_len as usize, 0);
    return packet;

    //    if it's a leap year, set bit 8:  csdw |= 1 << 8;
    // 3. words = bcd_doy_words(&parts)
    // 4. data_len = 4 (CSDW) + 6 (three u16 words) = 10
}



// ======================= Milestone 5 =======================
// Session plan: SESSION-milestone5.md

/// One UDP datagram from the Python sensor -> a Reading.
/// Wire format (Python: struct.pack("<IQff", ...)), 20 bytes, little-endian:
///   0..4 u32 index   4..12 u64 t_ns   12..16 f32 entropy   16..20 f32 top_prob
fn parse_datagram(buf: &[u8]) -> Option<Reading> {
    // YOUR PART 9: the reverse of what you did for packets: bytes -> numbers.
    // 1. if buf.len() != DATAGRAM_BYTES, return None
    if buf.len() != DATAGRAM_BYTES {
        return None;
    }
    let index = u32::from_le_bytes(buf[0..4].try_into().unwrap());
    let t_ns = u64::from_le_bytes(buf[4..12].try_into().unwrap());
    let entropy = f32::from_le_bytes(buf[12..16].try_into().unwrap());
    let top_prob = f32::from_le_bytes(buf[16..20].try_into().unwrap());
    return Some(Reading { index, t_ns, entropy, top_prob });
    //
    // Done when: cargo test --bin ch10write datagram_
    //todo!("parse_datagram")
}

/// Turns readings into Ch10 bytes one at a time, the way a live recorder does.
/// CSV mode and UDP mode both use it, so they produce identical files.
struct Recorder {
    anchor_ns: Option<u64>, // None until the first reading arrives
    seq: u8,                // next sequence number on channel 0
}

impl Recorder {
    fn new() -> Recorder {
        Recorder { anchor_ns: None, seq: 1 } // channel 0 used seq 0 for TMATS
    }

    /// The bytes to write for one reading. The first reading also brings the
    /// TMATS and time packets; every reading after that is one packet.
    fn packets_for(&mut self, r: &Reading) -> Vec<u8> {
        // YOUR PART 10: your milestone 4 main logic, one reading at a time.
        let mut out = Vec::new();
        if self.anchor_ns.is_none() {
            let a = r.t_ns / 1_000_000_000 * 1_000_000_000; // round down to whole second
            self.anchor_ns = Some(a);
            out.extend_from_slice(&build_tmats_packet(TMATS, 0, 0));
            out.extend_from_slice(&build_time_packet(a, 0, 0));
        } 
        // 1. let mut out = Vec::new();
        // 2. if self.anchor_ns.is_none() {   (this is the first reading)
        //        a = r.t_ns rounded DOWN to a whole second
        //        self.anchor_ns = Some(a);
        //        append build_tmats_packet(TMATS, 0, 0) to out
        //        append build_time_packet(a, 0, 0) to out    <- RTC 0 = the anchor
        //    }
        let anchor = self.anchor_ns.unwrap();
        let rtc = rtc_from_ns(r.t_ns, anchor);
        out.extend_from_slice(&build_confidence_packet(r, self.seq, rtc));
        self.seq = self.seq.wrapping_add(1);
        return out;
        // 3. let anchor = self.anchor_ns.unwrap();
        // 4. rtc = rtc_from_ns(r.t_ns, anchor)    <- count from the ANCHOR, not the first token
        // 5. append build_confidence_packet(r, self.seq, rtc); then
        //    self.seq = self.seq.wrapping_add(1);
        // 6. return out
        //
        // Done when: cargo test --bin ch10write recorder_
        //todo!("packets_for")
    }
}

fn print_anchor(anchor_ns: u64) {
    let a = utc_parts(anchor_ns);
    eprintln!("time anchor: {} day {} {:02}:{:02}:{:02} UTC", a.year, a.doy, a.hour, a.min, a.sec);
}

/// Pre-filled: wait for the next datagram. Returns its length, or None on timeout.
fn recv_datagram(sock: &UdpSocket, buf: &mut [u8]) -> Option<usize> {
    match sock.recv_from(buf) {
        Ok((n, _from)) => Some(n),
        // Linux reports a timeout as WouldBlock, Windows as TimedOut
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock
            || e.kind() == std::io::ErrorKind::TimedOut => None,
        Err(e) => panic!("UDP receive failed: {e}"),
    }
}

/// Live mode: receive readings over UDP and write each one to disk as it arrives,
/// like a recorder. Returns how many readings were recorded.
fn record(sock: &UdpSocket, out: &str) -> usize {
    let mut file = fs::File::create(out).expect("could not create output file");
    let mut rec = Recorder::new();
    let mut buf = [0u8; 64];
    let mut expected: u32 = 0;
    let mut count = 0;

    loop {
        // Pre-filled: wait for the next datagram. None means 5 s of silence.
        let Some(n) = recv_datagram(sock, &mut buf) else {
            eprintln!("no data for 5 s, stopping");
            break;
        };
        if n == 0 {
            break; // an empty datagram means the sender finished the run
        }

        // YOUR PART 11: handle one datagram.
        // 1. let Some(r) = parse_datagram(&buf[..n]) else { warn and continue };
        let Some(r) = parse_datagram(&buf[..n]) else {
            eprintln!("warning: bad datagram, ignoring");
            continue;
        };
        if r.index != expected {
            eprintln!("gap: expected reading {expected}, got {}", r.index);
            expected = r.index + 1;
        }
        file.write_all(&rec.packets_for(&r)).expect("write failed");
        if count == 0 {
            print_anchor(rec.anchor_ns.unwrap());
            sock.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        }
        eprintln!("rx {:4}  entropy {:5.2}  top {:4.2}", r.index, r.entropy, r.top_prob);
        count += 1;

        // 2. if r.index != expected, eprintln!("gap: expected reading {expected}, got {}", r.index)
        //    (UDP can drop datagrams, and a recorder should say so), then expected = r.index + 1
        // 3. file.write_all(&rec.packets_for(&r)).expect("write failed");
        //    Written right away, so the file is valid even if the run gets cut off.
        // 4. if count == 0 (first reading):
        //        print_anchor(rec.anchor_ns.unwrap());
        //        sock.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        //    Start the silence timer only now, because the model can take a while to load.
        // 5. eprintln!("rx {:4}  entropy {:5.2}  top {:4.2}", r.index, r.entropy, r.top_prob);
        //    count += 1;
        //
        // Done when: cargo test --bin ch10write udp_
    }
    count
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // Live mode: ch10write --udp <port> <out.c10>
    if args.len() == 4 && args[1] == "--udp" {
        let port: u16 = args[2].parse().expect("bad port number");
        let sock = UdpSocket::bind(("0.0.0.0", port)).expect("could not open UDP port");
        eprintln!("listening on UDP port {port}, recording to {}", args[3]);
        let n = record(&sock, &args[3]);
        eprintln!("recorded {n} readings to {}", args[3]);
        return;
    }

    if args.len() != 3 {
        eprintln!("usage: ch10write <confidence.csv> <out.c10>");
        eprintln!("       ch10write --udp <port> <out.c10>");
        std::process::exit(1);
    }

    // CSV mode: the same Recorder as live mode, so both make identical files.
    let readings = read_readings(&args[1]);
    let mut rec = Recorder::new();
    let mut file = Vec::new();
    for r in &readings {
        file.extend_from_slice(&rec.packets_for(r));
    }
    if let Some(a) = rec.anchor_ns {
        print_anchor(a);
    }
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

    // ---------------- Milestone 4: time packet ----------------

    #[test]
    fn utc_parts_known_time() {
        // First token of confidence_sample.csv: 2026-11-20 06:17:01.998 UTC
        let p = utc_parts(1795155421998010048);
        assert_eq!((p.year, p.doy), (2026, 324));
        assert_eq!((p.hour, p.min, p.sec, p.ms), (6, 17, 1, 998));
    }

    #[test]
    fn leap_years() {
        assert!(is_leap(2024) && is_leap(2000));
        assert!(!is_leap(2026) && !is_leap(1900));
    }

    // These two come straight from time packets in the sample files.
    // If your BCD matches a real recorder's, it's right.
    #[test]
    fn bcd_matches_discrete_c10() {
        // discrete.c10: day 22, 21:19:58.000
        let p = UtcParts { year: 0, doy: 22, hour: 21, min: 19, sec: 58, ms: 0 };
        assert_eq!(bcd_doy_words(&p), [0x5800, 0x2119, 0x0022]);
    }

    #[test]
    fn bcd_matches_event_c10() {
        // event.c10: day 131, 22:16:28.000
        let p = UtcParts { year: 0, doy: 131, hour: 22, min: 16, sec: 28, ms: 0 };
        assert_eq!(bcd_doy_words(&p), [0x2800, 0x2216, 0x0131]);
    }

    #[test]
    fn bcd_packs_milliseconds() {
        // 5.370 s: tens of ms = 7 (bits 3-0), hundreds of ms = 3 (bits 7-4)
        let p = UtcParts { year: 0, doy: 1, hour: 0, min: 0, sec: 5, ms: 370 };
        assert_eq!(bcd_doy_words(&p)[0], 0x0537);
    }

    #[test]
    fn time_packet_layout() {
        // The whole-second anchor for confidence_sample.csv: day 324, 06:17:01
        let p = build_time_packet(1795155421_000_000_000, 0, 0);

        assert_eq!(p.len(), 36); // 24 header + 4 CSDW + 6 time + 2 filler
        assert_eq!(u16::from_le_bytes([p[2], p[3]]), TIME_CHANNEL);
        assert_eq!(u32::from_le_bytes([p[4], p[5], p[6], p[7]]), 36); // packet length
        assert_eq!(u32::from_le_bytes([p[8], p[9], p[10], p[11]]), 10); // data length
        assert_eq!(p[13], 0); // first packet on channel 1
        assert_eq!(p[15], TYPE_TIME);
        assert_eq!(header_checksum(&p[..24]), u16::from_le_bytes([p[22], p[23]]));

        assert_eq!(u32::from_le_bytes([p[24], p[25], p[26], p[27]]), 0x30); // CSDW, 2026 not leap
        assert_eq!(u16::from_le_bytes([p[28], p[29]]), 0x0100); // 01 s
        assert_eq!(u16::from_le_bytes([p[30], p[31]]), 0x0617); // 06:17
        assert_eq!(u16::from_le_bytes([p[32], p[33]]), 0x0324); // day 324
        assert_eq!(&p[34..36], &[0, 0]); // filler
    }

    #[test]
    fn time_packet_sets_leap_year_bit() {
        // 2024-03-01 00:00:00 UTC, a leap year
        let p = build_time_packet(1709251200_000_000_000, 0, 0);
        assert_eq!(u32::from_le_bytes([p[24], p[25], p[26], p[27]]), 0x130);
    }

    // ---------------- Milestone 5: live streaming ----------------

    // Builds the same 20 bytes Python's struct.pack("<IQff", ...) sends.
    fn datagram(index: u32, t_ns: u64, entropy: f32, top_prob: f32) -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(&index.to_le_bytes());
        d.extend_from_slice(&t_ns.to_le_bytes());
        d.extend_from_slice(&entropy.to_le_bytes());
        d.extend_from_slice(&top_prob.to_le_bytes());
        d
    }

    fn rtc_of(p: &[u8]) -> u64 {
        u64::from_le_bytes([p[16], p[17], p[18], p[19], p[20], p[21], 0, 0])
    }

    #[test]
    fn datagram_parses() {
        let r = parse_datagram(&datagram(6, 1795155422181235564, 3.811_142, 0.136_583)).unwrap();
        assert_eq!((r.index, r.t_ns), (6, 1795155422181235564));
        assert_eq!((r.entropy, r.top_prob), (3.811_142, 0.136_583));
    }

    #[test]
    fn datagram_wrong_length_is_rejected() {
        assert!(parse_datagram(&[0u8; 19]).is_none());
        assert!(parse_datagram(&[0u8; 21]).is_none());
    }

    // These two pin the timing. A file can pass every byte count in ch10scan
    // and still map readings to the wrong clock time.
    #[test]
    fn recorder_first_reading_brings_headers() {
        let mut rec = Recorder::new();
        let r = Reading { index: 0, t_ns: 1795155421998010048, entropy: 0.5, top_prob: 0.9 };
        let out = rec.packets_for(&r);
        let tmats_len = build_tmats_packet(TMATS, 0, 0).len();
        assert_eq!(out.len(), tmats_len + 36 + 40); // TMATS + time + one reading

        let time = &out[tmats_len..tmats_len + 36];
        assert_eq!(time[15], TYPE_TIME);
        assert_eq!(rtc_of(time), 0); // RTC 0 = the whole-second anchor

        let conf = &out[tmats_len + 36..];
        assert_eq!(conf[13], 1); // seq 1 on channel 0
        assert_eq!(rtc_of(conf), 9_980_100); // first token is 0.998 s AFTER the anchor
    }

    #[test]
    fn recorder_later_readings_are_one_packet() {
        let mut rec = Recorder::new();
        rec.packets_for(&Reading { index: 0, t_ns: 1795155421998010048, entropy: 0.5, top_prob: 0.9 });
        let out = rec.packets_for(&Reading { index: 1, t_ns: 1795155422057528658, entropy: 0.5, top_prob: 0.9 });
        assert_eq!(out.len(), 40);
        assert_eq!(out[13], 2); // seq 2
        assert_eq!(rtc_of(&out), 10_575_286); // 1.0575 s after the anchor
    }

    #[test]
    fn udp_records_live_readings() {
        // A real UDP round trip on this machine: a receiver thread and a sender.
        let rx = UdpSocket::bind("127.0.0.1:0").unwrap();
        let addr = rx.local_addr().unwrap();
        let out = std::env::temp_dir().join("ch10write_udp_test.c10");
        let out_path = out.to_str().unwrap().to_string();
        let receiver = std::thread::spawn(move || record(&rx, &out_path));

        let tx = UdpSocket::bind("127.0.0.1:0").unwrap();
        let times = [1795155421998010048u64, 1795155422057528658, 1795155422083224414];
        for (i, t) in times.iter().enumerate() {
            tx.send_to(&datagram(i as u32, *t, 1.0, 0.5), addr).unwrap();
        }
        tx.send_to(&[], addr).unwrap(); // end of run

        assert_eq!(receiver.join().unwrap(), 3);
        let bytes = fs::read(&out).unwrap();
        assert_eq!(bytes.len(), build_tmats_packet(TMATS, 0, 0).len() + 36 + 3 * 40);
    }
}
