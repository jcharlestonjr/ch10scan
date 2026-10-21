use std::env;
use std::fs;
use std::collections::BTreeMap;
use std::fmt;

#[derive(Default)]
struct Stats {
    packets: u32,
    bytes: u64,
}

#[derive(Debug, PartialEq)]
enum HeaderError {
    TooShort(usize),
    BadSync(u16),
    BadChecksum { stored: u16, computed: u16 },
    BadLength(u32),
}

#[derive(Debug)]
struct PrimaryHeader {
    channel_id: u16,
    packet_len: u32,
    data_len: u32,
    seq: u8,
    flags: u8,
    data_type: u8,
    rtc: u64,
    checksum: u16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Family {
    ComputerGenerated,
    Pcm,
    Time,
    Mil1553,
    Analog,
    Discrete,
    Unknown,
}

impl Family {
    fn from_code(code: u8) -> Family {
        match code >> 3 {
            0 => Family::ComputerGenerated,
            1 => Family::Pcm,
            2 => Family::Time,
            3 => Family::Mil1553,
            4 => Family::Analog,
            5 => Family::Discrete,
            _ => Family::Unknown,
        }
    }
}

impl fmt::Display for Family {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            Family::ComputerGenerated => "Computer Generated",
            Family::Pcm => "PCM",
            Family::Time => "Time",
            Family::Mil1553 => "1553",
            Family::Analog => "Analog",
            Family::Discrete => "Discrete",
            Family::Unknown => "Unknown",
        };
        f.pad(name)
    }
}

fn parse_header(h: &[u8]) -> Result<PrimaryHeader, HeaderError> {
    if h.len() < 24 {
        return Err(HeaderError::TooShort(h.len()));
    }

    let sync = u16::from_le_bytes([h[0], h[1]]);
    if sync != 0xEB25 {
        return Err(HeaderError::BadSync(sync));
    }

    let channel_id = u16::from_le_bytes([h[2], h[3]]);
    let packet_len = u32::from_le_bytes([h[4], h[5], h[6], h[7]]);
    let data_len = u32::from_le_bytes([h[8], h[9], h[10], h[11]]);
    let seq = h[13];
    let flags = h[14];
    let data_type = h[15];
    let rtc = u64::from_le_bytes([h[16], h[17], h[18], h[19],h[20], h[21], 0, 0]);
    let checksum = u16::from_le_bytes([h[22],h[23]]);

    let computed = header_checksum(h);
    if computed != checksum {
        return Err(HeaderError::BadChecksum { stored: checksum, computed });
    }
    if packet_len < 24 {
        return Err(HeaderError::BadLength(packet_len));
    }


    Ok(PrimaryHeader { channel_id, packet_len, data_len, seq, flags, data_type, rtc, checksum })
}


fn header_checksum(h: &[u8]) -> u16 {
    let mut sum: u16 = 0;
    for i in (0..22).step_by(2) {
        // YOUR PART: read the little-endian word from h[i], h[i + 1]
        // and add it to sum, wrapping past 0xFFFF
        let check = u16::from_le_bytes([h[i], h[i + 1]]);
        sum = sum.wrapping_add(check);
    }
    sum
}

fn find_next_packet(bytes: &[u8], from: usize) -> Option<usize> {
    // YOUR PART: return the first offset at or after `from` where
    // parse_header succeeds, or None if there isn't one
    for i in from..bytes.len() {
        if parse_header(&bytes[i..]).is_ok() {
            return Some(i);
        }
    }
    None
}

fn tmats_text(packet: &[u8], hdr: &PrimaryHeader) -> Option<String> {
    // YOUR PART:
    // 1. hdr_end = 36 if flags bit 7 is set, otherwise 24
    let hdr_end = if hdr.flags & 0x80 != 0 { 36 } else { 24 };
    // 2. data_end = hdr_end + data_len
    let data_end = hdr_end + hdr.data_len as usize;
    // 3. return None if data_len < 4 or data_end > packet.len()
    if hdr.data_len < 4 || data_end > packet.len() {
        return None;
    }
    // 4. the text is packet[hdr_end + 4 .. data_end]
    let text: &[u8] = &packet[hdr_end + 4..data_end];
    Some(String::from_utf8_lossy(text).into_owned())
}


fn main() {
    let path = env::args().nth(1).expect("usage: ch10scan <file>");
    let bytes = fs::read(&path).expect("could not read file");

    let mut offset = 0;

    let mut stats: BTreeMap<(u16, u8), Stats> = BTreeMap::new();
    let mut skipped: usize = 0;
    while offset < bytes.len() {
       
        let hdr = match parse_header(&bytes[offset..]) {
            Ok(hdr) => hdr,
           Err(e) => {
                eprintln!("{e:x?} at {offset:08x}");
                match find_next_packet(&bytes, offset + 1) {
                    Some(next) => {
                        eprintln!("  resynced at {next:08x}, skipped {} bytes", next - offset);
                        skipped += next - offset;
                        offset = next;
                        continue;
                    }
                    None => {
                        skipped += bytes.len() - offset;
                        break;
                    }
                }
            }
        };

        let len = hdr.packet_len as usize;
        let left = bytes.len() - offset;
        if offset + len > bytes.len() {
            eprintln!("packet at {offset:08x}: needs {len} bytes, only {left} left");
            skipped += left;
            break;
        }
        
        if hdr.data_type == 0x01 {
            let packet = &bytes[offset..offset + len];
            match tmats_text(packet, &hdr) {
                Some(text) => {
                    fs::write("tmats.txt", &text).expect("could not write tmats.txt");
                    eprintln!("TMATS: {} bytes -> tmats.txt", text.len());
                }
                None => eprintln!("TMATS packet at {offset:08x}: bad data length"),
            }
        }

        let s = stats.entry((hdr.channel_id, hdr.data_type)).or_default();
        s.packets += 1;
        s.bytes += hdr.packet_len as u64;


        offset += hdr.packet_len as usize;
    }

    for ((ch,dt), s) in &stats {
        println!("ch {:3}  type 0x{:02x}  {:<18} F{}  packets {:3}  bytes {}",
         ch, dt, Family::from_code(*dt), dt & 0x07, s.packets, s.bytes);
    }
    let packet_bytes: u64 = stats.values().map(|s| s.bytes).sum();
    let accounted = packet_bytes + skipped as u64;
    if accounted == bytes.len() as u64 {
        println!("OK: all {} bytes accounted for ({skipped} skipped)", bytes.len());
    } else {
        eprintln!("MISMATCH: {accounted} accounted vs {} in file", bytes.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Packet 1 from discrete.c10 — the header you decoded by hand
    const TMATS_HEADER: [u8; 24] = [
        0x25, 0xeb, 0x00, 0x00, 0x00, 0x6e, 0x00, 0x00,
        0xb8, 0x43, 0x00, 0x00, 0x05, 0x00, 0x00, 0x01,
        0x25, 0x0a, 0xa3, 0xb8, 0x06, 0x00, 0xb0, 0x60,
    ];

    #[test]
    fn checksum_matches_known_header() {
        assert_eq!(header_checksum(&TMATS_HEADER), 0x60b0);
    }

    #[test]
    fn parses_known_header() {
        let hdr = parse_header(&TMATS_HEADER).unwrap();
        assert_eq!(hdr.channel_id, 0);
        assert_eq!(hdr.packet_len, 28160);
        assert_eq!(hdr.data_len, 17336);
        assert_eq!(hdr.data_type, 1);
        assert_eq!(hdr.rtc, 28867496485);
    }

    #[test]
    fn rejects_corrupted_byte() {
        let mut bad = TMATS_HEADER;
        bad[2] = 0x07;
        assert!(parse_header(&bad).is_err());
    }

    // YOUR PART: two more tests
    //   1. a slice shorter than 24 bytes is rejected
    #[test]
    fn rejects_too_short_slice() {
        assert_eq!(
            parse_header(&[]).unwrap_err(),
            HeaderError::TooShort(0)
        );
    }

    //   2. a header with a bad sync word is rejected
    #[test]
    fn rejects_bad_sync_word() {
        let mut bad = TMATS_HEADER;
        bad[0] = 0x00;
        bad[1] = 0x00;
        assert_eq!(
            parse_header(&bad).unwrap_err(),
            HeaderError::BadSync(0x0000)
        );
    }

    #[test]
    fn finds_packet_after_junk() {
        let mut buf = vec![0xAA; 5];
        buf.extend_from_slice(&TMATS_HEADER);
        assert_eq!(find_next_packet(&buf, 0), Some(5));
    }
}