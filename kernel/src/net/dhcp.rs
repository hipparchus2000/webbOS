//! DHCP (Dynamic Host Configuration Protocol)
//!
//! Client for automatic IP configuration.

use alloc::vec;
use alloc::vec::Vec;

use crate::net::{Ipv4Address, Port, IpProtocol, udp, NetworkConfig};
use crate::println;

/// DHCP ports
const DHCP_CLIENT_PORT: Port = Port::new(68);
const DHCP_SERVER_PORT: Port = Port::new(67);

/// DHCP message types
const DHCP_DISCOVER: u8 = 1;
const DHCP_OFFER: u8 = 2;
const DHCP_REQUEST: u8 = 3;
const DHCP_DECLINE: u8 = 4;
const DHCP_ACK: u8 = 5;
const DHCP_NAK: u8 = 6;
const DHCP_RELEASE: u8 = 7;

/// DHCP packet
#[repr(C)]
struct DhcpPacket {
    op: u8,
    htype: u8,
    hlen: u8,
    hops: u8,
    xid: u32,
    secs: u16,
    flags: u16,
    ciaddr: [u8; 4],
    yiaddr: [u8; 4],
    siaddr: [u8; 4],
    giaddr: [u8; 4],
    chaddr: [u8; 16],
    sname: [u8; 64],
    file: [u8; 128],
    // Magic cookie: 0x63825363
    // Options follow
}

/// DHCP options
const OPT_MESSAGE_TYPE: u8 = 53;
const OPT_SUBNET_MASK: u8 = 1;
const OPT_ROUTER: u8 = 3;
const OPT_DNS: u8 = 6;
const OPT_REQUESTED_IP: u8 = 50;
const OPT_SERVER_ID: u8 = 54;
const OPT_END: u8 = 255;

/// Current DHCP state
#[derive(Debug, Clone, Copy)]
enum DhcpState {
    Idle,
    Selecting,
    Requesting,
    Bound,
}

static mut DHCP_STATE: DhcpState = DhcpState::Idle;
static mut DHCP_XID: u32 = 0x12345678;

/// Start DHCP discovery
pub fn start_dhcp() {
    println!("[dhcp] Starting DHCP discovery...");

    unsafe {
        DHCP_STATE = DhcpState::Selecting;
        DHCP_XID = 0x12345678;
    }

    // Bind DHCP client port
    let _ = udp::bind(DHCP_CLIENT_PORT);

    // Send DHCP discover
    send_discover();
}

/// Send DHCP discover
fn send_discover() {
    let mut packet = vec![0u8; 300];

    // Fill DHCP header
    packet[0] = 1; // BOOTREQUEST
    packet[1] = 1; // Ethernet
    packet[2] = 6; // MAC length
    packet[3] = 0; // Hops
    
    // XID
    unsafe {
        packet[4..8].copy_from_slice(&DHCP_XID.to_be_bytes());
    }
    
    // secs, flags
    packet[8..10].copy_from_slice(&[0, 0]);
    packet[10..12].copy_from_slice(&[0x80, 0x00]); // Broadcast flag

    // Client IP (0.0.0.0)
    packet[12..16].fill(0);
    
    // Your IP (0.0.0.0)
    packet[16..20].fill(0);
    
    // Server IP (0.0.0.0)
    packet[20..24].fill(0);
    
    // Gateway IP (0.0.0.0)
    packet[24..28].fill(0);

    // Client MAC (TODO: use actual MAC)
    packet[28..34].copy_from_slice(&[0x52, 0x54, 0x00, 0x12, 0x34, 0x56]);
    packet[34..44].fill(0); // Padding to 16 bytes

    // Server name (empty)
    packet[44..108].fill(0);
    
    // Boot file (empty)
    packet[108..236].fill(0);

    // Magic cookie
    packet[236..240].copy_from_slice(&[0x63, 0x82, 0x53, 0x63]);

    // Options
    let mut opt_pos = 240;
    
    // Message type: Discover
    packet[opt_pos] = OPT_MESSAGE_TYPE;
    packet[opt_pos + 1] = 1;
    packet[opt_pos + 2] = DHCP_DISCOVER;
    opt_pos += 3;

    // Client identifier (MAC)
    packet[opt_pos] = 61;
    packet[opt_pos + 1] = 7;
    packet[opt_pos + 2] = 1; // Ethernet
    packet[opt_pos + 3..opt_pos + 9].copy_from_slice(&[0x52, 0x54, 0x00, 0x12, 0x34, 0x56]);
    opt_pos += 9;

    // Parameter request list
    packet[opt_pos] = 55;
    packet[opt_pos + 1] = 4;
    packet[opt_pos + 2] = OPT_SUBNET_MASK;
    packet[opt_pos + 3] = OPT_ROUTER;
    packet[opt_pos + 4] = OPT_DNS;
    packet[opt_pos + 5] = 15; // Domain name
    opt_pos += 6;

    // End
    packet[opt_pos] = OPT_END;

    // Send broadcast
    let _ = udp::send_to(
        DHCP_CLIENT_PORT,
        Ipv4Address::broadcast(),
        DHCP_SERVER_PORT,
        &packet[..opt_pos + 1]
    );

    println!("[dhcp] Sent DISCOVER");
}

/// Send DHCP request
fn send_request(offer: &DhcpOffer) {
    let mut packet = vec![0u8; 300];

    // Fill DHCP header
    packet[0] = 1; // BOOTREQUEST
    packet[1] = 1; // Ethernet
    packet[2] = 6; // MAC length
    packet[3] = 0; // Hops
    
    unsafe {
        packet[4..8].copy_from_slice(&DHCP_XID.to_be_bytes());
    }
    
    packet[8..12].copy_from_slice(&[0, 0, 0x80, 0x00]); // Broadcast
    packet[12..28].fill(0); // IPs
    
    // Client MAC
    packet[28..34].copy_from_slice(&[0x52, 0x54, 0x00, 0x12, 0x34, 0x56]);
    packet[34..44].fill(0);
    packet[44..236].fill(0);

    // Magic cookie
    packet[236..240].copy_from_slice(&[0x63, 0x82, 0x53, 0x63]);

    // Options
    let mut opt_pos = 240;

    // Message type: Request
    packet[opt_pos] = OPT_MESSAGE_TYPE;
    packet[opt_pos + 1] = 1;
    packet[opt_pos + 2] = DHCP_REQUEST;
    opt_pos += 3;

    // Requested IP
    packet[opt_pos] = OPT_REQUESTED_IP;
    packet[opt_pos + 1] = 4;
    packet[opt_pos + 2..opt_pos + 6].copy_from_slice(offer.ip.as_bytes());
    opt_pos += 6;

    // Server ID
    packet[opt_pos] = OPT_SERVER_ID;
    packet[opt_pos + 1] = 4;
    packet[opt_pos + 2..opt_pos + 6].copy_from_slice(offer.server.as_bytes());
    opt_pos += 6;

    // End
    packet[opt_pos] = OPT_END;

    let _ = udp::send_to(
        DHCP_CLIENT_PORT,
        Ipv4Address::broadcast(),
        DHCP_SERVER_PORT,
        &packet[..opt_pos + 1]
    );

    println!("[dhcp] Sent REQUEST for {:?}", offer.ip);

    unsafe {
        DHCP_STATE = DhcpState::Requesting;
    }
}

/// DHCP offer
struct DhcpOffer {
    ip: Ipv4Address,
    server: Ipv4Address,
    subnet_mask: Ipv4Address,
    gateway: Ipv4Address,
    dns: Ipv4Address,
}

/// Process DHCP packet
pub fn process_dhcp_packet(data: &[u8]) {
    if data.len() < 240 {
        return;
    }

    let state = unsafe { DHCP_STATE };

    match state {
        DhcpState::Selecting => {
            // Looking for DHCPOFFER
            if let Some(offer) = parse_offer(data) {
                println!("[dhcp] Received OFFER from {:?}", offer.server);
                send_request(&offer);
            }
        }
        DhcpState::Requesting => {
            // Looking for DHCPACK
            if parse_ack(data) {
                println!("[dhcp] Received ACK - configuration complete");
                unsafe {
                    DHCP_STATE = DhcpState::Bound;
                }
            }
        }
        _ => {}
    }
}

/// Parse DHCP offer
fn parse_offer(data: &[u8]) -> Option<DhcpOffer> {
    // Check XID
    let xid = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    unsafe {
        if xid != DHCP_XID {
            return None;
        }
    }

    // Get offered IP (yiaddr)
    let ip = Ipv4Address::new([data[16], data[17], data[18], data[19]]);

    // Parse options
    let mut pos = 240;
    let mut message_type = 0u8;
    let mut server_ip = Ipv4Address::unspecified();
    let mut subnet_mask = Ipv4Address::from_octets(255, 255, 255, 0);
    let mut gateway = Ipv4Address::unspecified();
    let mut dns = Ipv4Address::unspecified();

    while pos < data.len() && data[pos] != OPT_END {
        let opt = data[pos];
        if opt == 0 {
            pos += 1;
            continue;
        }

        if pos + 1 >= data.len() {
            break;
        }
        let len = data[pos + 1] as usize;

        if pos + 2 + len > data.len() {
            break;
        }

        match opt {
            OPT_MESSAGE_TYPE => {
                message_type = data[pos + 2];
            }
            OPT_SUBNET_MASK => {
                if len == 4 {
                    subnet_mask = Ipv4Address::new([
                        data[pos + 2], data[pos + 3], data[pos + 4], data[pos + 5]
                    ]);
                }
            }
            OPT_ROUTER => {
                if len >= 4 {
                    gateway = Ipv4Address::new([
                        data[pos + 2], data[pos + 3], data[pos + 4], data[pos + 5]
                    ]);
                }
            }
            OPT_DNS => {
                if len >= 4 {
                    dns = Ipv4Address::new([
                        data[pos + 2], data[pos + 3], data[pos + 4], data[pos + 5]
                    ]);
                }
            }
            OPT_SERVER_ID => {
                if len == 4 {
                    server_ip = Ipv4Address::new([
                        data[pos + 2], data[pos + 3], data[pos + 4], data[pos + 5]
                    ]);
                }
            }
            _ => {}
        }

        pos += 2 + len;
    }

    if message_type != DHCP_OFFER {
        return None;
    }

    Some(DhcpOffer {
        ip,
        server: server_ip,
        subnet_mask,
        gateway,
        dns,
    })
}

/// Parse DHCP ACK
fn parse_ack(data: &[u8]) -> bool {
    // Check XID
    let xid = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    unsafe {
        if xid != DHCP_XID {
            return false;
        }
    }

    // Get assigned IP
    let ip = Ipv4Address::new([data[16], data[17], data[18], data[19]]);

    // Parse options
    let mut pos = 240;
    let mut message_type = 0u8;
    let mut subnet_mask = Ipv4Address::from_octets(255, 255, 255, 0);
    let mut gateway = Ipv4Address::unspecified();
    let mut dns = Ipv4Address::unspecified();

    while pos < data.len() && data[pos] != OPT_END {
        let opt = data[pos];
        if opt == 0 {
            pos += 1;
            continue;
        }

        if pos + 1 >= data.len() {
            break;
        }
        let len = data[pos + 1] as usize;

        match opt {
            OPT_MESSAGE_TYPE => {
                message_type = data[pos + 2];
            }
            OPT_SUBNET_MASK => {
                if len == 4 {
                    subnet_mask = Ipv4Address::new([
                        data[pos + 2], data[pos + 3], data[pos + 4], data[pos + 5]
                    ]);
                }
            }
            OPT_ROUTER => {
                if len >= 4 {
                    gateway = Ipv4Address::new([
                        data[pos + 2], data[pos + 3], data[pos + 4], data[pos + 5]
                    ]);
                }
            }
            OPT_DNS => {
                if len >= 4 {
                    dns = Ipv4Address::new([
                        data[pos + 2], data[pos + 3], data[pos + 4], data[pos + 5]
                    ]);
                }
            }
            _ => {}
        }

        pos += 2 + len;
    }

    if message_type != DHCP_ACK {
        return false;
    }

    // Apply configuration
    let config = NetworkConfig {
        ip,
        netmask: subnet_mask,
        gateway,
        dns,
    };
    super::set_config(config);

    true
}

/// Check if DHCP is bound
pub fn is_bound() -> bool {
    unsafe {
        matches!(DHCP_STATE, DhcpState::Bound)
    }
}
