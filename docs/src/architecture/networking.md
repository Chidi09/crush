# eBPF Networks and Bridges

Container networking in Crush is engineered for maximum throughput and low latency, bypassing standard, slow iptables-based packet processing paths where possible.

---

## Virtual Interfaces and Bridges

When a network-isolated container is spawned:
1. **Virtual Ethernet Pairs (veth)**: Crush creates a bidirectional virtual ethernet pipe. One end is attached to the host, and the other is placed inside the container's isolated network namespace (`CLONE_NEWNET`).
2. **Software Bridge**: Host-side endpoints are bound to a secure software bridge interface (typically named `crush0`), which enables cross-container communication.
3. **Network Address Translation (NAT)**: Outbound traffic from the bridge to the internet is handled natively through network address translation (NAT) utilizing system tables or direct sockets.

---

## eBPF Packet Routing

On modern Linux systems, Crush incorporates **eBPF (Extended Berkeley Packet Filter)** programs directly into the network architecture:

- **Bypassing the TCP/IP Stack**: Normally, a packet traveling between two local containers must traverse the full kernel routing and filtering stack twice. eBPF socket maps (`sockmap`) intercept data streams directly at the socket layer and forward them immediately to the destination socket, achieving near-native loopback speeds.
- **Micro-segmentation**: High-performance, zero-latency firewall rules are attached as eBPF programs on traffic control (TC) hooks, allowing instant filtering of container packets before they consume kernel CPU cycles.
