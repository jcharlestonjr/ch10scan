// ch10tail: a live quick-look display for a Ch10 file that is still being written.
// Milestone 6 (Ground Station Ridge). Session plan: SESSION-milestone6.md
//
// usage: ch10tail <file.c10>          follow the file as it grows (Ctrl+C to stop)
//        ch10tail <file.c10> --once   show what's there now and exit
//
// A recorder writes whole packets, but a reader polling a growing file can catch one
// half-written. The key idea here: only ever hand back COMPLETE packets, and come back
// for the rest next time.

#![allow(unused_variables, dead_code)] // delete this line once all YOUR PARTs are done

use std::env;
use std::fs;
use std::thread::sleep;
use std::time::Duration;

const SYNC: u16 = 0xEB25;
const TYPE_USER_DEFINED: u8 = 0x00;
const TYPE_TMATS: u8 = 0x01;
const TYPE_TIME: u8 = 0x11;
const PROBE_CHANNEL: u16 = 2; // milestone 8's injection probe
const BAR_WIDTH: usize = 40;

/// One complete packet: the header fields a display needs, plus the body.
struct Packet {
    channel: u16,
    data_type: u8,
    rtc: u64,
    body: Vec<u8>, // data_len bytes right after the 24-byte header (CSDW first)
}

// Copied from ch10write.rs: your checksum, unchanged.
fn header_checksum(h: &[u8]) -> u16 {
    let mut sum: u16 = 0;
    for i in (0..22).step_by(2) {
        sum = sum.wrapping_add(u16::from_le_bytes([h[i], h[i + 1]]));
    }
    sum
}

/// The complete packet starting at `offset`, and where the next one starts.
/// None if there isn't a whole, valid packet there (yet).
fn next_packet(data: &[u8], offset: usize) -> Option<(Packet, usize)> {
    // YOUR PART 12: read one packet, but only if all of it has arrived.
    // 1. if fewer than 24 bytes are left after offset, return None
    // 2. h = &data[offset..offset + 24]; the sync word (bytes 0-1) must be SYNC
    // 3. the stored checksum (bytes 22-23) must equal header_checksum(h)
    // 4. packet_len = bytes 4..8, data_len = bytes 8..12 (u32 little-endian)
    // 5. if offset + packet_len > data.len(), return None    <- the live-file case:
    //    the recorder hasn't finished writing this packet, so wait for more
    // 6. build the Packet: channel = bytes 2..4, data_type = byte 15,
    //    rtc = bytes 16..22 (6 bytes; pad to 8 for u64::from_le_bytes),
    //    body = data[offset + 24 .. offset + 24 + data_len].to_vec()
    // 7. return Some((packet, offset + packet_len))
    //
    // Done when: cargo test --bin ch10tail tail_packet
    todo!("next_packet")
}

/// A confidence reading from a channel 0 user-defined (F0) packet:
/// body = CSDW (4 bytes), u32 index, f32 entropy, f32 top_prob.
fn decode_confidence(p: &Packet) -> Option<(u32, f32, f32)> {
    // YOUR PART 13: the reverse of your build_confidence_packet.
    // 1. only channel 0, data_type TYPE_USER_DEFINED, and a body of at least 16 bytes;
    //    anything else returns None
    // 2. skip the 4-byte CSDW, then read index (u32), entropy (f32), top_prob (f32)
    //
    // Done when: cargo test --bin ch10tail tail_confidence
    todo!("decode_confidence")
}

/// Day of year, hour, minute, second from a Time F1 packet body (day-of-year format).
fn decode_time(p: &Packet) -> Option<(u32, u32, u32, u32)> {
    // YOUR PART 14: your bcd_doy_words, run backwards.
    // 1. only data_type TYPE_TIME with a body of at least 10 bytes (CSDW + 3 words)
    // 2. w0, w1, w2 = the u16 words at body[4..6], [6..8], [8..10]
    // 3. pull each digit out with a shift and a mask, e.g. units of seconds = (w0 >> 8) & 0xF
    //      seconds = tens (bits 14-12) * 10 + units (bits 11-8)               of w0
    //      minutes = tens (bits 6-4) * 10 + units (bits 3-0)                  of w1
    //      hours   = tens (bits 13-12) * 10 + units (bits 11-8)               of w1
    //      day     = hundreds (bits 9-8) * 100 + tens (7-4) * 10 + units (3-0) of w2
    // 4. return Some((day, hours, minutes, seconds))
    //
    // Done when: cargo test --bin ch10tail tail_time
    todo!("decode_time")
}

/// One line of the strip chart, e.g. "   6 |##############################          | 3.00"
fn strip_line(index: u32, entropy: f32, full_scale: f32) -> String {
    // YOUR PART 15: draw one reading.
    // 1. n = how many of BAR_WIDTH cells to fill: (entropy / full_scale * BAR_WIDTH),
    //    rounded, and clamped to 0..=BAR_WIDTH (an over-range reading pins at full scale)
    // 2. format!("{:4} |{}{}| {:.2}", index, "#".repeat(n), " ".repeat(BAR_WIDTH - n), entropy)
    //
    // Done when: cargo test --bin ch10tail tail_strip
    todo!("strip_line")
}

/// A probe reading from the channel 2 user-defined packet (milestone 8). Pre-filled.
fn decode_probe(p: &Packet) -> Option<(u32, f32)> {
    if p.channel != PROBE_CHANNEL || p.data_type != TYPE_USER_DEFINED || p.body.len() < 12 {
        return None;
    }
    let pos = u32::from_le_bytes(p.body[4..8].try_into().unwrap());
    let score = f32::from_le_bytes(p.body[8..12].try_into().unwrap());
    Some((pos, score))
}

/// Clock time for an RTC, given the time packet's anchor. Pre-filled.
fn clock_at(anchor: Option<((u32, u32, u32, u32), u64)>, rtc: u64) -> String {
    let Some(((day, h, m, s), rtc0)) = anchor else {
        return "--:--:--.---".to_string();
    };
    let ms_total = ((h * 3600 + m * 60 + s) as u64) * 1000 + rtc.saturating_sub(rtc0) / 10_000;
    let (hh, mm, ss, ms) = (ms_total / 3_600_000 % 24, ms_total / 60_000 % 60, ms_total / 1000 % 60, ms_total % 1000);
    format!("{hh:02}:{mm:02}:{ss:02}.{ms:03}")
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: ch10tail <file.c10> [--once]");
        std::process::exit(1);
    }
    let path = &args[1];
    let once = args.iter().any(|a| a == "--once");

    let mut offset = 0;
    let mut anchor = None;
    let mut readings = 0;
    let mut waiting_said = false;
    loop {
        let data = fs::read(path).unwrap_or_default(); // the recorder may not have created it yet
        while let Some((p, next)) = next_packet(&data, offset) {
            match p.data_type {
                TYPE_TMATS => println!("TMATS received ({} bytes)", p.body.len()),
                TYPE_TIME => {
                    if let Some(t) = decode_time(&p) {
                        println!("time: day {} {:02}:{:02}:{:02} UTC at RTC {}", t.0, t.1, t.2, t.3, p.rtc);
                        anchor = Some((t, p.rtc));
                    }
                }
                _ => {
                    if let Some((index, entropy, _top)) = decode_confidence(&p) {
                        println!("{} entropy {}", clock_at(anchor, p.rtc), strip_line(index, entropy, 4.0));
                        readings += 1;
                    } else if let Some((pos, score)) = decode_probe(&p) {
                        let flag = if score >= 0.5 { "  <- injection?" } else { "" };
                        println!("{} probe   {}{}", clock_at(anchor, p.rtc), strip_line(pos, score, 1.0), flag);
                    }
                }
            }
            offset = next;
        }
        if once {
            break;
        }
        if offset < data.len() && data.len() - offset >= 24 && next_packet(&data, offset).is_none() && !waiting_said {
            eprintln!("(waiting for the rest of a packet at byte {offset})");
            waiting_said = true;
        }
        sleep(Duration::from_millis(100));
    }
    eprintln!("{readings} readings, {offset} of {} bytes read", fs::metadata(path).map(|m| m.len()).unwrap_or(0));
}

#[cfg(test)]
mod tests {
    use super::*;

    // samples/ai_sample.c10 was written by YOUR ch10write from confidence_sample.csv.
    const SAMPLE: &[u8] = include_bytes!("../../samples/ai_sample.c10");

    fn all_packets(data: &[u8]) -> (Vec<Packet>, usize) {
        let mut out = Vec::new();
        let mut off = 0;
        while let Some((p, next)) = next_packet(data, off) {
            out.push(p);
            off = next;
        }
        (out, off)
    }

    #[test]
    fn tail_packet_reads_whole_file() {
        let (packets, end) = all_packets(SAMPLE);
        assert_eq!(end, SAMPLE.len());
        assert_eq!(packets.len(), 60); // TMATS + time + 58 readings
        assert_eq!(packets[0].data_type, TYPE_TMATS);
        assert_eq!((packets[1].channel, packets[1].data_type), (1, TYPE_TIME));
        assert_eq!(packets[2].rtc, 9_980_100);
    }

    #[test]
    fn tail_packet_waits_for_partial_packet() {
        let (_, end) = all_packets(SAMPLE);
        let last_start = end - 40; // the last confidence packet is 40 bytes
        assert!(next_packet(&SAMPLE[..end - 1], last_start).is_none());
        assert!(next_packet(&SAMPLE[..last_start + 10], last_start).is_none());
        assert!(next_packet(SAMPLE, last_start).is_some());
    }

    #[test]
    fn tail_packet_rejects_bad_sync_and_checksum() {
        let mut bad = SAMPLE.to_vec();
        bad[0] ^= 0xFF;
        assert!(next_packet(&bad, 0).is_none());
        let mut bad = SAMPLE.to_vec();
        bad[5] ^= 0x01; // packet length byte, so the checksum no longer matches
        assert!(next_packet(&bad, 0).is_none());
    }

    #[test]
    fn tail_confidence_decodes_reading_6() {
        let (packets, _) = all_packets(SAMPLE);
        let (index, entropy, top) = decode_confidence(&packets[8]).unwrap();
        assert_eq!(index, 6);
        assert!((entropy - 3.811_142).abs() < 1e-5);
        assert!((top - 0.136_583).abs() < 1e-5);
        assert!(decode_confidence(&packets[0]).is_none()); // TMATS isn't a reading
    }

    #[test]
    fn tail_time_decodes_anchor() {
        let (packets, _) = all_packets(SAMPLE);
        assert_eq!(decode_time(&packets[1]), Some((324, 6, 17, 1)));
        assert_eq!(decode_time(&packets[0]), None);
    }

    #[test]
    fn tail_strip_line_draws_bars() {
        assert_eq!(strip_line(6, 3.0, 4.0), format!("   6 |{}{}| 3.00", "#".repeat(30), " ".repeat(10)));
        assert_eq!(strip_line(0, 0.0, 4.0), format!("   0 |{}| 0.00", " ".repeat(40)));
        assert_eq!(strip_line(12, 9.5, 4.0), format!("  12 |{}| 9.50", "#".repeat(40))); // pinned
    }
}
