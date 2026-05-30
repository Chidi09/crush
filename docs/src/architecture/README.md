# Architecture & Mechanics

This section delves deep into the internals and system engineering design of **Crush**. 

Crush is built entirely in Rust and designed to be a **daemonless, native container runtime**. It bypasses the overhead of heavy daemon processes by directly utilizing operating system kernel boundaries.

Explore the deep technical mechanics behind Crush:

- **[Zero-Config Stack Detection](stack_detection.md)**: How Crush finger-prints source code repositories dynamically.
- **[Linux Namespace Isolation](linux_isolation.md)**: Containerization internals on Linux systems using namespaces and cgroups v2.
- **[Windows Job Objects Sandbox](windows_isolation.md)**: Native container containment on Windows using lightweight Job Objects.
- **[macOS Apple Virtualization](macos_isolation.md)**: Bootstrapping lightweight guest microVMs on macOS.
- **[WASM and WASI Preview 2 Integration](wasm_runtime.md)**: Embedded WASM compilation and secure execution with Wasmtime.
- **[eBPF Networks and Bridges](networking.md)**: High-performance container networking.
- **[OCI Content-Addressed Store](image_store.md)**: Secure local image registry, compression, and deduplicated layer storage.
- **[Smart Stacktrace Intercept & AI Diagnosis](error_diagnostics.md)**: Zero-config compiler and runtime crash analysis.
