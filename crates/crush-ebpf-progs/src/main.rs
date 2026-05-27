#![no_std]
#![no_main]

use aya_bpf::{
    bindings::xdp_action,
    macros::{map, xdp, classifier},
    maps::HashMap,
    programs::{XdpContext, TcContext},
};
use aya_log_ebpf::info;

// ---------------------------------------------------------------------------
// BPF Maps
// ---------------------------------------------------------------------------

/// Map: container destination IPv4 (network byte order u32) → veth ifindex.
/// Used by the XDP program to redirect inbound packets directly to the
/// container's veth interface without going through the full Linux stack.
#[map]
static CONTAINER_IPS: HashMap<u32, u32> = HashMap::with_max_entries(4096, 0);

/// Map: host TCP/UDP destination port (u16 cast to u32 key) → packed u64
/// encoding (container_ip_be << 32 | container_port_be).
/// Used by the TC egress DNAT program.
#[map]
static PORT_MAPPINGS: HashMap<u32, u64> = HashMap::with_max_entries(4096, 0);

// ---------------------------------------------------------------------------
// XDP: xdp_router
// ---------------------------------------------------------------------------
//
// Fast-path packet routing: if the destination IPv4 belongs to a known
// container (present in CONTAINER_IPS), redirect the packet to the container's
// veth ifindex.  All other traffic is passed through untouched.

#[xdp]
pub fn xdp_router(ctx: XdpContext) -> u32 {
    match try_xdp_router(&ctx) {
        Ok(action) => action,
        Err(_) => xdp_action::XDP_PASS,
    }
}

#[inline(always)]
fn try_xdp_router(ctx: &XdpContext) -> Result<u32, ()> {
    let data = ctx.data();
    let data_end = ctx.data_end();

    // Need at least Ethernet (14) + IPv4 header (20) = 34 bytes
    if data + 34 > data_end {
        return Ok(xdp_action::XDP_PASS);
    }

    // Ethertype is at Ethernet offset 12 (big-endian u16)
    let ethertype = unsafe {
        let ptr = (data + 12) as *const u16;
        u16::from_be(*ptr)
    };

    // Only process IPv4 (0x0800)
    if ethertype != 0x0800 {
        return Ok(xdp_action::XDP_PASS);
    }

    // Destination IPv4 is at Ethernet (14) + IP dst offset (16) = byte 30
    let dst_ip = unsafe {
        let ptr = (data + 30) as *const u32;
        *ptr // already in network byte order
    };

    if let Some(ifindex) = unsafe { CONTAINER_IPS.get(&dst_ip) } {
        // Redirect frame to the container's veth ifindex
        let ifindex_val = *ifindex;
        info!(ctx, "xdp_router: redirecting dst {} to ifindex {}", dst_ip, ifindex_val);
        return Ok(unsafe {
            aya_bpf::helpers::bpf_redirect(ifindex_val, 0) as u32
        });
    }

    Ok(xdp_action::XDP_PASS)
}

// ---------------------------------------------------------------------------
// TC Egress: tc_egress  (DNAT — rewrite host-port → container IP:port)
// ---------------------------------------------------------------------------
//
// Attached on the host-side bridge/veth egress qdisc.  When a TCP or UDP
// packet leaves the host toward a port that has a container mapping, the
// destination IP and destination port are rewritten in-place and the IPv4
// checksum is recalculated using bpf_l3_csum_replace.

#[classifier]
pub fn tc_egress(ctx: TcContext) -> i32 {
    match try_tc_egress(&ctx) {
        Ok(ret) => ret,
        Err(_) => 0, // TC_ACT_OK — pass on error
    }
}

#[inline(always)]
fn try_tc_egress(ctx: &TcContext) -> Result<i32, ()> {
    let data = ctx.data();
    let data_end = ctx.data_end();

    // Ethernet (14) + IPv4 min (20) + TCP/UDP dst port at offset 2 inside L4
    // Minimum needed: 14 + 20 + 4 = 38 bytes
    if data + 38 > data_end {
        return Ok(0); // TC_ACT_OK
    }

    // Ethertype
    let ethertype = unsafe {
        let ptr = (data + 12) as *const u16;
        u16::from_be(*ptr)
    };

    if ethertype != 0x0800 {
        return Ok(0);
    }

    // IPv4 protocol field is at byte 14 + 9 = 23
    let proto = unsafe { *((data + 23) as *const u8) };

    // Only handle TCP (6) and UDP (17)
    if proto != 6 && proto != 17 {
        return Ok(0);
    }

    // The IPv4 header length (IHL) in 32-bit words is the low nibble of byte 14
    let ihl = unsafe { *((data + 14) as *const u8) & 0x0F };
    let ip_hdr_len = (ihl as usize) * 4;
    let l4_start = 14 + ip_hdr_len;

    // L4 destination port is at l4_start + 2
    if l4_start + 4 > data_end - data {
        return Ok(0);
    }

    let dst_port = unsafe {
        let ptr = (data + l4_start + 2) as *const u16;
        u16::from_be(*ptr) as u32
    };

    if let Some(mapping) = unsafe { PORT_MAPPINGS.get(&dst_port) } {
        let val = *mapping;
        // Upper 32 bits: container IP in network byte order
        // Lower 32 bits: container port (only low 16 bits meaningful)
        let container_ip_be = (val >> 32) as u32;
        let container_port_be = ((val & 0xFFFF) as u16).to_be();

        // Pointer to the current destination IP in the IPv4 header (offset 30)
        let old_dst_ip = unsafe {
            let ptr = (data + 30) as *const u32;
            *ptr
        };

        // Rewrite destination IP
        unsafe {
            let dst_ip_ptr = (data + 30) as *mut u32;
            *dst_ip_ptr = container_ip_be;
        }

        // Rewrite destination port in L4 header
        unsafe {
            let dst_port_ptr = (data + l4_start + 2) as *mut u16;
            *dst_port_ptr = container_port_be;
        }

        // Recalculate IPv4 header checksum using bpf_l3_csum_replace
        // Checksum field is at Ethernet (14) + IPv4 checksum offset (10) = 24
        let csum_offset = (14 + 10) as i64;
        unsafe {
            aya_bpf::helpers::bpf_l3_csum_replace(
                ctx.skb.skb as *mut _,
                csum_offset as u32,
                old_dst_ip as u64,
                container_ip_be as u64,
                4,
            );
        }

        info!(ctx, "tc_egress: DNAT port {} → {}:{}", dst_port, container_ip_be, container_port_be);
    }

    Ok(0) // TC_ACT_OK
}

// ---------------------------------------------------------------------------
// Metrics: per-cgroup network bytes
// ---------------------------------------------------------------------------
//
// Attached as cgroup_skb on INGRESS and EGRESS for each container cgroup.
// Key: cgroup_id (u64). Value: cumulative bytes as u64.

#[map]
static NET_BYTES_MAP: HashMap<u64, u64> = HashMap::with_max_entries(4096, 0);

#[map]
static NET_TX_MAP: HashMap<u64, u64> = HashMap::with_max_entries(4096, 0);

#[aya_bpf::macros::cgroup_skb]
pub fn crush_net_ingress(ctx: aya_bpf::programs::SkBuffContext) -> i32 {
    let cgroup_id = unsafe { aya_bpf::helpers::bpf_get_current_cgroup_id() };
    let len = ctx.len() as u64;
    if let Some(val) = unsafe { NET_BYTES_MAP.get_ptr_mut(&cgroup_id) } {
        unsafe { *val += len; }
    } else {
        let _ = unsafe { NET_BYTES_MAP.insert(&cgroup_id, &len, 0) };
    }
    1
}

#[aya_bpf::macros::cgroup_skb]
pub fn crush_net_egress(ctx: aya_bpf::programs::SkBuffContext) -> i32 {
    let cgroup_id = unsafe { aya_bpf::helpers::bpf_get_current_cgroup_id() };
    let len = ctx.len() as u64;
    if let Some(val) = unsafe { NET_TX_MAP.get_ptr_mut(&cgroup_id) } {
        unsafe { *val += len; }
    } else {
        let _ = unsafe { NET_TX_MAP.insert(&cgroup_id, &len, 0) };
    }
    1
}

// ---------------------------------------------------------------------------
// Metrics: per-cgroup block I/O
// ---------------------------------------------------------------------------

#[map]
static BLOCK_READ_MAP: HashMap<u64, u64> = HashMap::with_max_entries(4096, 0);

#[map]
static BLOCK_WRITE_MAP: HashMap<u64, u64> = HashMap::with_max_entries(4096, 0);

#[aya_bpf::macros::tracepoint]
pub fn crush_block_rq_complete(ctx: aya_bpf::programs::TracePointContext) -> u32 {
    let cgroup_id = unsafe { aya_bpf::helpers::bpf_get_current_cgroup_id() };
    let nr_sector: u32 = unsafe { ctx.read_at(20).unwrap_or(0) };
    let bytes = (nr_sector as u64) * 512;
    let rwbs_byte: u8 = unsafe { ctx.read_at(24).unwrap_or(b'?') };

    match rwbs_byte {
        b'R' => {
            if let Some(val) = unsafe { BLOCK_READ_MAP.get_ptr_mut(&cgroup_id) } {
                unsafe { *val += bytes; }
            } else {
                let _ = unsafe { BLOCK_READ_MAP.insert(&cgroup_id, &bytes, 0) };
            }
        }
        b'W' | b'F' => {
            if let Some(val) = unsafe { BLOCK_WRITE_MAP.get_ptr_mut(&cgroup_id) } {
                unsafe { *val += bytes; }
            } else {
                let _ = unsafe { BLOCK_WRITE_MAP.insert(&cgroup_id, &bytes, 0) };
            }
        }
        _ => {}
    }
    0
}

// ---------------------------------------------------------------------------
// Panic handler (required for no_std)
// ---------------------------------------------------------------------------

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
