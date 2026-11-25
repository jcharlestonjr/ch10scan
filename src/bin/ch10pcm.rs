// ch10pcm: put the AI channels into PCM, the format every telemetry ground station speaks.
// Milestone 9 (The PCM Citadel). Session plan: SESSION-milestone9.md
//
// usage: ch10pcm <ai.c10> <out_pcm.c10>     convert a recording from ch10write
//        ch10pcm --decom <out_pcm.c10>      decommutate it back, like a ground station would
//
// Each confidence reading becomes one PCM minor frame: a 32-bit sync word, then 16-bit words.
// The asynchronous probe channel is sampled-and-held into each frame, the way a PCM encoder
// samples any input. Frames go into PCM Data Format 1 packets in packed mode, laid out the
// same way as the packed channel in pcm.c10 from a real recorder.

#![allow(unused_variables, dead_code)] // delete this line once all YOUR PARTs are done

use std::env;
use std::fs;

const SYNC: u16 = 0xEB25;
const DATA_TYPE_VERSION: u8 = 0x07;
const TYPE_USER_DEFINED: u8 = 0x00;
const TYPE_TMATS: u8 = 0x01;
const TYPE_PCM: u8 = 0x09; // PCM Data Format 1
const TYPE_TIME: u8 = 0x11;
const PCM_CHANNEL: u16 = 3;
const FRAME_SYNC: [u16; 2] = [0xFE6B, 0x2840]; // 32-bit sync 0xFE6B2840, same as pcm.c10
// CSDW for packed mode, the same bits pcm.c10's packed channel uses:
// IPH (30), MA (29), MI (28), major and minor frame lock (27-24 all set), PM packed mode (19).
const PCM_CSDW_PACKED: u32 = 0x7F08_0000;
const MINOR_FRAME_STATUS: u16 = 0xF000; // intra-packet data header: frame lock bits, as in pcm.c10
const WORDS: usize = 7; // 2 sync + counter + index + entropy + top_prob + probe
const FRAME_BYTES: usize = 8 + 2 + 2 * WORDS; // time stamp + status + words = 24
const FRAMES_PER_PACKET: usize = 16;
const ENTROPY_FULL_SCALE: f32 = 65.535; // 1 count = 0.001 nats
const UNIT_FULL_SCALE: f32 = 1.0; // probabilities and probe scores

type Frame = (u64, [u16; WORDS]); // (RTC, words)

/// Engineering units -> a 16-bit count, 0 at zero and 65535 at full scale.
fn to_counts(value: f32, full_scale: f32) -> u16 {
    // YOUR PART 27: what an analog-to-digital converter does.
    // 1. counts = value / full_scale * 65535, rounded to the nearest whole count
    // 2. clamp to 0..=65535 (anything over range pins at the top, negatives at 0)
    // 3. return it as u16
    //
    // Done when: cargo test --bin ch10pcm pcm_counts
    todo!("to_counts")
}

/// The reverse, for the ground station side. Pre-filled.
fn to_value(counts: u16, full_scale: f32) -> f32 {
    counts as f32 / 65535.0 * full_scale
}

/// One minor frame: sync, frame counter, token index, then the three channels.
fn minor_frame(counter: u16, index: u32, entropy: f32, top_prob: f32, probe: f32) -> [u16; WORDS] {
    // YOUR PART 28: commutate one sample of every channel into a frame.
    // word 0-1: FRAME_SYNC[0], FRAME_SYNC[1]
    // word 2:   counter (lets the ground station spot dropped frames)
    // word 3:   index, truncated to 16 bits (index as u16)
    // word 4:   to_counts(entropy, ENTROPY_FULL_SCALE)
    // word 5:   to_counts(top_prob, UNIT_FULL_SCALE)
    // word 6:   to_counts(probe, UNIT_FULL_SCALE)
    //
    // Done when: cargo test --bin ch10pcm pcm_minor_frame
    todo!("minor_frame")
}

// Copied from ch10write.rs: your checksum and header builder, unchanged.
fn header_checksum(h: &[u8]) -> u16 {
    let mut sum: u16 = 0;
    for i in (0..22).step_by(2) {
        sum = sum.wrapping_add(u16::from_le_bytes([h[i], h[i + 1]]));
    }
    sum
}

fn build_header(channel: u16, packet_len: u32, data_len: u32, seq: u8, data_type: u8, rtc: u64) -> [u8; 24] {
    let mut h = [0u8; 24];
    h[0..2].copy_from_slice(&SYNC.to_le_bytes());
    h[2..4].copy_from_slice(&channel.to_le_bytes());
    h[4..8].copy_from_slice(&packet_len.to_le_bytes());
    h[8..12].copy_from_slice(&data_len.to_le_bytes());
    h[12] = DATA_TYPE_VERSION;
    h[13] = seq;
    h[14] = 0;
    h[15] = data_type;
    h[16..22].copy_from_slice(&rtc.to_le_bytes()[..6]);
    let ck = header_checksum(&h);
    h[22..24].copy_from_slice(&ck.to_le_bytes());
    h
}

/// A PCM Data Format 1 packet (packed mode) holding one or more minor frames.
fn build_pcm_packet(frames: &[Frame], seq: u8) -> Vec<u8> {
    // YOUR PART 29: the packed-mode layout from pcm.c10.
    // 1. data_len = 4 (CSDW) + frames.len() * FRAME_BYTES
    //    packet_len = 24 + data_len, rounded up to a multiple of 4
    // 2. header: build_header(PCM_CHANNEL, packet_len, data_len, seq, TYPE_PCM, rtc of the FIRST frame)
    // 3. PCM_CSDW_PACKED (u32 LE)
    // 4. for each (rtc, words) frame:
    //      intra-packet time stamp: the 6 low bytes of rtc, then 2 zero bytes (8 bytes total)
    //      intra-packet data header: MINOR_FRAME_STATUS (u16 LE)
    //      each word as u16 LE
    // 5. resize to packet_len and return
    //
    // Done when: cargo test --bin ch10pcm pcm_packet
    todo!("build_pcm_packet")
}

/// Ground-station side: pull the frames back out of a PCM packet body (CSDW first).
fn decom(body: &[u8]) -> Vec<Frame> {
    // YOUR PART 30: decommutate.
    // 1. start just after the 4-byte CSDW; while a whole FRAME_BYTES chunk is left:
    //      rtc   = the first 6 bytes of the chunk (pad to 8 for u64::from_le_bytes)
    //      words = WORDS u16 values starting 10 bytes into the chunk (after time stamp + status)
    //      if words[0..2] != FRAME_SYNC, stop: this isn't a frame (out of sync)
    //      push (rtc, words); move ahead FRAME_BYTES
    // 2. return the frames
    //
    // Done when: cargo test --bin ch10pcm pcm_decom
    todo!("decom")
}

// ---------- Pre-filled: reading Ch10 files, TMATS, and the two modes ----------

struct Packet {
    channel: u16,
    data_type: u8,
    rtc: u64,
    raw: Vec<u8>,  // the whole packet, header included
    body: Vec<u8>, // data_len bytes after the header
}

fn read_packets(data: &[u8]) -> Vec<Packet> {
    let mut out = Vec::new();
    let mut off = 0;
    while off + 24 <= data.len() {
        let h = &data[off..off + 24];
        if u16::from_le_bytes([h[0], h[1]]) != SYNC || header_checksum(h) != u16::from_le_bytes([h[22], h[23]]) {
            break;
        }
        let plen = u32::from_le_bytes(h[4..8].try_into().unwrap()) as usize;
        let dlen = u32::from_le_bytes(h[8..12].try_into().unwrap()) as usize;
        if off + plen > data.len() {
            break;
        }
        out.push(Packet {
            channel: u16::from_le_bytes([h[2], h[3]]),
            data_type: h[15],
            rtc: u64::from_le_bytes([h[16], h[17], h[18], h[19], h[20], h[21], 0, 0]),
            raw: data[off..off + plen].to_vec(),
            body: data[off + 24..off + 24 + dlen].to_vec(),
        });
        off += plen;
    }
    out
}

fn tmats(bit_rate: u32) -> String {
    let lines = [
        "G\\PN:AI-TELEMETRY".to_string(),
        "G\\106:15".into(),
        "G\\COM:AI channels commutated into PCM by ch10pcm. Words after sync: 2 frame counter, 3 token index, \
4 entropy (1 count = 0.001 nats), 5 top probability (counts/65535), 6 probe score (counts/65535)".into(),
        "G\\DSI\\N:1".into(),
        "G\\DSI-1:AIREC".into(),
        "G\\DST-1:OTH".into(),
        "R-1\\ID:AIREC".into(),
        "R-1\\N:2".into(),
        "R-1\\TK1-1:1".into(), "R-1\\DSI-1:TIME".into(), "R-1\\TK4-1:1".into(), "R-1\\CHE-1:T".into(),
        "R-1\\CDT-1:TIMEIN".into(), "R-1\\CDLN-1:TIME".into(), "R-1\\TTF-1:1".into(),
        format!("R-1\\TK1-2:{PCM_CHANNEL}"), "R-1\\DSI-2:AI-PCM".into(), format!("R-1\\TK4-2:{PCM_CHANNEL}"),
        "R-1\\CHE-2:T".into(), "R-1\\CDT-2:PCMIN".into(), "R-1\\CDLN-2:AI-PCM".into(),
        "R-1\\PDTF-2:1".into(), "R-1\\PDP-2:PFS".into(),
        "P-1\\DLN:AI-PCM".into(), "P-1\\D1:NRZ-L".into(), format!("P-1\\D2:{bit_rate}"),
        "P-1\\D4:N".into(), "P-1\\D7:N".into(), "P-1\\D8:N/A".into(), "P-1\\TF:ONE".into(),
        "P-1\\F1:16".into(), "P-1\\F2:M".into(), "P-1\\F3:NO".into(),
        "P-1\\MF\\N:1".into(), format!("P-1\\MF1:{}", WORDS - 1), format!("P-1\\MF2:{}", 32 + 16 * (WORDS - 2)),
        "P-1\\MF4:32".into(), "P-1\\MF5:11111110011010110010100001000000".into(),
        "P-1\\SYNC1:0".into(), "P-1\\SYNC2:1".into(), "P-1\\SYNC3:0".into(), "P-1\\SYNC4:1".into(),
        "P-1\\ISF\\N:0".into(),
    ];
    lines.iter().map(|l| format!("{l};\n")).collect()
}

fn tmats_packet(text: &str) -> Vec<u8> {
    let data_len = (4 + text.len()) as u32;
    let packet_len = (24 + data_len + 3) / 4 * 4;
    let mut p = build_header(0, packet_len, data_len, 0, TYPE_TMATS, 0).to_vec();
    p.extend_from_slice(&0x0Bu32.to_le_bytes()); // CH10VER for 106-15, ASCII TMATS
    p.extend_from_slice(text.as_bytes());
    p.resize(packet_len as usize, 0);
    p
}

fn convert(input: &str, output: &str) {
    let packets = read_packets(&fs::read(input).expect("could not read input"));
    let time = packets.iter().find(|p| p.data_type == TYPE_TIME).expect("no time packet in the input");
    let mut probes: Vec<(u64, f32)> = packets.iter()
        .filter(|p| p.channel == 2 && p.data_type == TYPE_USER_DEFINED && p.body.len() >= 12)
        .map(|p| (p.rtc, f32::from_le_bytes(p.body[8..12].try_into().unwrap())))
        .collect();
    probes.sort_by_key(|(rtc, _)| *rtc);

    let mut frames: Vec<Frame> = Vec::new();
    let mut probe_now = 0.0; // sample-and-hold: the latest probe score at or before this frame
    let mut next_probe = 0;
    for p in packets.iter().filter(|p| p.channel == 0 && p.data_type == TYPE_USER_DEFINED && p.body.len() >= 16) {
        while next_probe < probes.len() && probes[next_probe].0 <= p.rtc {
            probe_now = probes[next_probe].1;
            next_probe += 1;
        }
        let index = u32::from_le_bytes(p.body[4..8].try_into().unwrap());
        let entropy = f32::from_le_bytes(p.body[8..12].try_into().unwrap());
        let top = f32::from_le_bytes(p.body[12..16].try_into().unwrap());
        frames.push((p.rtc, minor_frame(frames.len() as u16, index, entropy, top, probe_now)));
    }
    if frames.is_empty() {
        eprintln!("no confidence readings in {input}");
        std::process::exit(1);
    }
    let span_s = (frames.last().unwrap().0 - frames[0].0).max(1) as f64 / 1e7;
    let bit_rate = ((frames.len() as f64 * (32 + 16 * (WORDS - 2)) as f64) / span_s).round() as u32;

    let mut out = tmats_packet(&tmats(bit_rate));
    out.extend_from_slice(&time.raw);
    for (seq, chunk) in frames.chunks(FRAMES_PER_PACKET).enumerate() {
        out.extend_from_slice(&build_pcm_packet(chunk, seq as u8));
    }
    fs::write(output, &out).expect("could not write output");
    eprintln!("{} frames in {} PCM packets, about {bit_rate} bit/s, {} bytes to {output}",
        frames.len(), frames.len().div_ceil(FRAMES_PER_PACKET), out.len());
}

fn decom_file(path: &str) {
    let packets = read_packets(&fs::read(path).expect("could not read file"));
    println!("frame  index  entropy  top_prob  probe");
    let mut n = 0;
    for p in packets.iter().filter(|p| p.data_type == TYPE_PCM) {
        for (_rtc, w) in decom(&p.body) {
            println!("{:5}  {:5}  {:7.3}  {:8.4}  {:5.3}", w[2], w[3],
                to_value(w[4], ENTROPY_FULL_SCALE), to_value(w[5], UNIT_FULL_SCALE), to_value(w[6], UNIT_FULL_SCALE));
            n += 1;
        }
    }
    eprintln!("{n} frames decommutated");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.as_slice() {
        [_, flag, path] if flag == "--decom" => decom_file(path),
        [_, input, output] => convert(input, output),
        _ => {
            eprintln!("usage: ch10pcm <ai.c10> <out_pcm.c10>\n       ch10pcm --decom <out_pcm.c10>");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pcm_counts_scale_and_clamp() {
        assert_eq!(to_counts(0.0, 1.0), 0);
        assert_eq!(to_counts(1.0, 1.0), 65535);
        assert_eq!(to_counts(0.5, 1.0), 32768); // 32767.5 rounds up
        assert_eq!(to_counts(3.811_142, ENTROPY_FULL_SCALE), 3811);
        assert_eq!(to_counts(70.0, ENTROPY_FULL_SCALE), 65535); // over range pins
        assert_eq!(to_counts(-0.2, 1.0), 0);
    }

    #[test]
    fn pcm_minor_frame_layout() {
        let f = minor_frame(9, 6, 3.811_142, 0.5, 0.0);
        assert_eq!(f, [0xFE6B, 0x2840, 9, 6, 3811, 32768, 0]);
        assert_eq!(minor_frame(0, 70_000, 0.0, 0.0, 1.0)[3], (70_000u32 & 0xFFFF) as u16);
    }

    #[test]
    fn pcm_packet_header_and_layout() {
        let frames = [(100u64, minor_frame(0, 0, 1.0, 0.5, 0.0)), (200, minor_frame(1, 1, 2.0, 0.5, 0.0))];
        let p = build_pcm_packet(&frames, 4);
        let data_len = 4 + 2 * FRAME_BYTES;
        assert_eq!(p.len(), (24 + data_len + 3) / 4 * 4);
        assert_eq!(u16::from_le_bytes([p[2], p[3]]), PCM_CHANNEL);
        assert_eq!(u32::from_le_bytes([p[8], p[9], p[10], p[11]]) as usize, data_len);
        assert_eq!((p[13], p[15]), (4, TYPE_PCM));
        assert_eq!(u64::from_le_bytes([p[16], p[17], p[18], p[19], p[20], p[21], 0, 0]), 100);
        assert_eq!(header_checksum(&p[..24]), u16::from_le_bytes([p[22], p[23]]));
        assert_eq!(u32::from_le_bytes([p[24], p[25], p[26], p[27]]), PCM_CSDW_PACKED);
        let second = 28 + FRAME_BYTES;
        assert_eq!(u64::from_le_bytes([p[second], p[second + 1], p[second + 2], p[second + 3], 0, 0, 0, 0]), 200);
    }

    #[test]
    fn pcm_packet_matches_real_recorder() {
        // The first 16 body bytes of the packed PCM channel (ch 55) in pcm.c10:
        // time stamp, frame status, 32-bit sync, then word 2 = 0x0001.
        const PCM_C10_PACKED: [u8; 16] = [
            0x5a, 0xdd, 0x0e, 0x11, 0x07, 0x00, 0x00, 0x00, 0x00, 0xf0, 0x6b, 0xfe, 0x40, 0x28, 0x01, 0x00,
        ];
        let p = build_pcm_packet(&[(0x07_110e_dd5a, minor_frame(1, 0, 0.0, 0.0, 0.0))], 0);
        assert_eq!(u32::from_le_bytes([p[24], p[25], p[26], p[27]]), 0x7F08_0000); // same CSDW as ch 55
        assert_eq!(p[28..44], PCM_C10_PACKED);
    }

    #[test]
    fn pcm_decom_round_trip() {
        let frames: Vec<Frame> = (0..5u16).map(|k| (1000 + k as u64, minor_frame(k, k as u32, 0.5 * k as f32, 0.9, 0.1))).collect();
        let p = build_pcm_packet(&frames, 0);
        let data_len = u32::from_le_bytes([p[8], p[9], p[10], p[11]]) as usize;
        assert_eq!(decom(&p[24..24 + data_len]), frames);
        let mut broken = p.clone();
        broken[28 + FRAME_BYTES + 10] ^= 0xFF; // corrupt the second frame's sync
        assert_eq!(decom(&broken[24..24 + data_len]).len(), 1);
    }
}
