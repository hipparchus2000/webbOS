//! DNS (Domain Name System)
//!
//! Simple DNS client for hostname resolution.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;

use crate::net::{Ipv4Address, Port, udp};
use crate::println;

/// DNS port
const DNS_PORT: Port = Port::new(53);

/// DNS opcodes
const DNS_OPCODE_QUERY: u16 = 0;

/// DNS response codes
const DNS_RCODE_NOERROR: u16 = 0;

/// DNS record types
const DNS_TYPE_A: u16 = 1;

/// DNS classes
const DNS_CLASS_IN: u16 = 1;

/// DNS header
#[repr(C)]
struct DnsHeader {
    id: u16,
    flags: u16,
    questions: u16,
    answer_rrs: u16,
    authority_rrs: u16,
    additional_rrs: u16,
}

impl DnsHeader {
    fn to_bytes(&self) -> [u8; 12] {
        let mut buf = [0u8; 12];
        buf[0..2].copy_from_slice(&self.id.to_be_bytes());
        buf[2..4].copy_from_slice(&self.flags.to_be_bytes());
        buf[4..6].copy_from_slice(&self.questions.to_be_bytes());
        buf[6..8].copy_from_slice(&self.answer_rrs.to_be_bytes());
        buf[8..10].copy_from_slice(&self.authority_rrs.to_be_bytes());
        buf[10..12].copy_from_slice(&self.additional_rrs.to_be_bytes());
        buf
    }

    fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 12 {
            return None;
        }

        Some(Self {
            id: u16::from_be_bytes([data[0], data[1]]),
            flags: u16::from_be_bytes([data[2], data[3]]),
            questions: u16::from_be_bytes([data[4], data[5]]),
            answer_rrs: u16::from_be_bytes([data[6], data[7]]),
            authority_rrs: u16::from_be_bytes([data[8], data[9]]),
            additional_rrs: u16::from_be_bytes([data[10], data[11]]),
        })
    }
}

/// DNS query state
struct DnsQuery {
    id: u16,
    name: String,
    result: Option<Ipv4Address>,
    completed: bool,
}

lazy_static! {
    static ref DNS_QUERIES: Mutex<Vec<DnsQuery>> = Mutex::new(Vec::new());
    static ref DNS_CACHE: Mutex<Vec<(String, Ipv4Address, u64)>> = Mutex::new(Vec::new());
    static ref NEXT_QUERY_ID: Mutex<u16> = Mutex::new(1);
}

/// Encode domain name
fn encode_name(name: &str) -> Vec<u8> {
    let mut result = Vec::new();
    
    for label in name.split('.') {
        result.push(label.len() as u8);
        result.extend_from_slice(label.as_bytes());
    }
    
    result.push(0); // Terminator
    result
}

/// Decode domain name from response
fn decode_name(data: &[u8], offset: usize) -> (String, usize) {
    let mut result = String::new();
    let mut pos = offset;
    let mut jumped = false;
    let mut jump_offset = 0;

    loop {
        if pos >= data.len() {
            break;
        }

        let len = data[pos] as usize;

        if len == 0 {
            pos += 1;
            break;
        }

        if len & 0xC0 == 0xC0 {
            // Compression pointer
            if !jumped {
                jump_offset = pos + 2;
            }
            pos = (((len & 0x3F) as usize) << 8) | (data[pos + 1] as usize);
            jumped = true;
            continue;
        }

        if !result.is_empty() {
            result.push('.');
        }

        pos += 1;
        if pos + len <= data.len() {
            result.push_str(core::str::from_utf8(&data[pos..pos + len]).unwrap_or(""));
        }
        pos += len;
    }

    (result, if jumped { jump_offset } else { pos })
}

/// Lookup hostname
pub fn lookup(hostname: &str) -> Option<Ipv4Address> {
    let config = super::get_config();
    if !config.is_configured() || config.dns.as_u32() == 0 {
        println!("[dns] No DNS server configured");
        return None;
    }

    // Check cache
    {
        let cache = DNS_CACHE.lock();
        for (name, ip, _) in cache.iter() {
            if name.eq_ignore_ascii_case(hostname) {
                return Some(*ip);
            }
        }
    }

    // Bind DNS client port
    let _ = udp::bind(Port::new(12345));

    // Build query
    let mut query_id = NEXT_QUERY_ID.lock();
    let id = *query_id;
    *query_id = id.wrapping_add(1);
    drop(query_id);

    let header = DnsHeader {
        id,
        flags: 0x0100, // Standard query, recursion desired
        questions: 1,
        answer_rrs: 0,
        authority_rrs: 0,
        additional_rrs: 0,
    };

    let name = encode_name(hostname);

    let mut query = vec![0u8; 12 + name.len() + 4];
    query[0..12].copy_from_slice(&header.to_bytes());
    query[12..12 + name.len()].copy_from_slice(&name);
    
    // QTYPE: A
    query[12 + name.len()..12 + name.len() + 2].copy_from_slice(&DNS_TYPE_A.to_be_bytes());
    // QCLASS: IN
    query[12 + name.len() + 2..12 + name.len() + 4].copy_from_slice(&DNS_CLASS_IN.to_be_bytes());

    // Send query
    if udp::send_to(Port::new(12345), config.dns, DNS_PORT, &query).is_err() {
        return None;
    }

    // Wait for response (simplified - should poll)
    // For now, register query and return None
    DNS_QUERIES.lock().push(DnsQuery {
        id,
        name: String::from(hostname),
        result: None,
        completed: false,
    });

    // Poll for response
    let mut buf = [0u8; 512];
    let start = crate::drivers::timer::elapsed_ms();
    
    while crate::drivers::timer::elapsed_ms() - start < 5000 {
        if let Some((_, _, len)) = udp::receive_from(Port::new(12345), &mut buf) {
            if let Some(ip) = parse_response(&buf[..len], id) {
                // Cache result
                DNS_CACHE.lock().push((
                    String::from(hostname),
                    ip,
                    crate::drivers::timer::elapsed_ms()
                ));
                return Some(ip);
            }
        }
    }

    None
}

/// Parse DNS response
fn parse_response(data: &[u8], expected_id: u16) -> Option<Ipv4Address> {
    let header = DnsHeader::from_bytes(data)?;

    if header.id != expected_id {
        return None;
    }

    // Check response code
    let rcode = header.flags & 0x0F;
    if rcode != DNS_RCODE_NOERROR {
        return None;
    }

    // Skip questions
    let mut pos = 12;
    for _ in 0..header.questions {
        while pos < data.len() && data[pos] != 0 {
            if data[pos] & 0xC0 == 0xC0 {
                pos += 2;
                break;
            }
            pos += 1 + (data[pos] as usize);
        }
        if pos < data.len() && data[pos] == 0 {
            pos += 1;
        }
        pos += 4; // QTYPE + QCLASS
    }

    // Parse answers
    for _ in 0..header.answer_rrs {
        if pos >= data.len() {
            break;
        }

        // Skip name
        let (_, new_pos) = decode_name(data, pos);
        pos = new_pos;

        if pos + 10 > data.len() {
            break;
        }

        let rtype = u16::from_be_bytes([data[pos], data[pos + 1]]);
        let rclass = u16::from_be_bytes([data[pos + 2], data[pos + 3]]);
        let _ttl = u32::from_be_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]]);
        let rdlen = u16::from_be_bytes([data[pos + 8], data[pos + 9]]) as usize;
        pos += 10;

        if rtype == DNS_TYPE_A && rclass == DNS_CLASS_IN && rdlen == 4 {
            if pos + 4 <= data.len() {
                return Some(Ipv4Address::new([
                    data[pos], data[pos + 1], data[pos + 2], data[pos + 3]
                ]));
            }
        }

        pos += rdlen;
    }

    None
}

/// Resolve hostname to IP address
pub fn resolve(hostname: &str) -> Option<Ipv4Address> {
    // Check if it's already an IP address
    if let Some(ip) = parse_ipv4(hostname) {
        return Some(ip);
    }

    lookup(hostname)
}

/// Parse IPv4 address from string
fn parse_ipv4(s: &str) -> Option<Ipv4Address> {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 4 {
        return None;
    }

    let mut bytes = [0u8; 4];
    for (i, part) in parts.iter().enumerate() {
        bytes[i] = part.parse().ok()?;
    }

    Some(Ipv4Address::new(bytes))
}
