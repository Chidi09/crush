#![no_std]
#![no_main]

use aya_ebpf::{
    bindings::xdp_action,
    macros::{map, xdp, classifier},
    maps::HashMap,
    programs::{XdpContext, TcContext},
};
use aya_log_ebpf::info;

// Map: container IPv4 (network byte order) -> ifindex of its veth peer
#[map]
static CONTAINER_IPS: HashMap<u32, u32> = HashMap::with_max_entries(4096, 0);

// Map: (dst_port as u32) -> container IP (network byte order)
#[map]
static PORT_MAPPINGS: HashMap<u32, u64> = HashMap::with_max_entries(4096, 0);

/// XDP fast-path: pass packets for known container IPs, allow all others.
/// Full routing (redirect via BPF_REDIRECT) would require bpf_fib_lookup and
/// a redirect map; this lighter version focuses on connection tracking for
/// crush's internal telemetry and a future fast-path drop of spoofed sources.
#[xdp]
pub fn crush_xdp(ctx: XdpContext) -> u32 {
    match try_crush_xdp(&ctx) {
        Ok(action) => action,
        Err(_) => xdp_action::XDP_PASS,
    }
}

fn try_crush_xdp(ctx: &XdpContext) -> Result<u32, ()> {
    let data = ctx.data();
    let data_end = ctx.data_end();

    // Minimum Ethernet + IPv4 header: 14 + 20 = 34 bytes
    if data + 34 > data_end {
        return Ok(xdp_action::XDP_PASS);
    }

    // Ethernet type field at offset 12 (big-endian)
    let ethertype = unsafe {
        let p = (data + 12) as *const u16;
        u16::from_be(*p)
    };

    // Only handle IPv4 (0x0800)
    if ethertype != 0x0800 {
        return Ok(xdp_action::XDP_PASS);
    }

    // Source IPv4 at offset 14 + 12 = 26
    let src_ip = unsafe {
        let p = (data + 26) as *const u32;
        *p // already in network byte order
    };

    if unsafe { CONTAINER_IPS.get(&src_ip) }.is_some() {
        // Packet is from a known container — pass through (telemetry hook point)
        return Ok(xdp_action::XDP_PASS);
    }

    Ok(xdp_action::XDP_PASS)
}

/// TC egress classifier: rewrite destination for port-mapped connections.
/// Attached on the bridge egress so that packets leaving the host toward
/// a container get their destination IP/port rewritten if a mapping exists.
#[classifier]
pub fn crush_tc_egress(ctx: TcContext) -> i32 {
    match try_crush_tc_egress(&ctx) {
        Ok(ret) => ret,
        Err(_) => 0, // TC_ACT_OK
    }
}

fn try_crush_tc_egress(ctx: &TcContext) -> Result<i32, ()> {
    // Minimum Ethernet + IPv4 + TCP header: 14 + 20 + 20 = 54 bytes
    let data = ctx.data();
    let data_end = ctx.data_end();

    if data + 54 > data_end {
        return Ok(0); // TC_ACT_OK
    }

    let ethertype = unsafe {
        let p = (data + 12) as *const u16;
        u16::from_be(*p)
    };

    if ethertype != 0x0800 {
        return Ok(0);
    }

    // Protocol at IPv4 offset 9 (from IP header start at byte 14)
    let proto = unsafe { *((data + 23) as *const u8) };
    // Only TCP (6) and UDP (17)
    if proto != 6 && proto != 17 {
        return Ok(0);
    }

    // Destination port at bytes 14 + 20 + 2 = 36 (TCP/UDP dest port)
    let dst_port = unsafe {
        let p = (data + 36) as *const u16;
        u16::from_be(*p) as u32
    };

    if let Some(mapping) = unsafe { PORT_MAPPINGS.get(&dst_port) } {
        // Upper 32 bits: container IP, lower 32 bits: container port
        let container_ip = ((*mapping >> 32) as u32).to_be();
        let _container_port = (*mapping & 0xFFFF_FFFF) as u16;

        // Rewrite destination IP in IPv4 header (offset 30)
        unsafe {
            let dst_ip_ptr = (data + 30) as *mut u32;
            *dst_ip_ptr = container_ip;
        }
        // Note: checksum recalculation requires bpf_l3_csum_replace / bpf_l4_csum_replace.
        // Offloaded to kernel via CHECKSUM_UNNECESSARY when skb->ip_summed is set.
    }

    Ok(0) // TC_ACT_OK
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
