# WASM and WASI Preview 2 Integration

Crush provides first-class, native WebAssembly (WASM) execution alongside standard OS process containers. This enables running ultra-portable, secure WASM modules with sub-millisecond start times and extremely low memory footprints.

---

## Embedded Wasmtime Engine

Rather than invoking an external runtime process, Crush compiles the state-of-the-art **Wasmtime** engine directly into its core Rust binary.

When a WASM module is detected or specified:
- **Direct Linkage**: The WASM binary is loaded and compiled directly into machine instructions in memory using Wasmtime's Cranelift JIT or loaded from pre-compiled AOT (.cwasm) files.
- **Instant Activation**: Starting a WASM instance takes less than **1 millisecond**, completely bypassing traditional OS kernel process spawn latency.
- **Minimal Sandbox Memory**: A running WASM instance requires as little as **2 MB** of memory, enabling thousands of concurrent sandboxes on a single machine.

---

## WASI Preview 2 Compatibility

Crush implements full compatibility with the **WebAssembly System Interface (WASI) Preview 2** specification, offering standardized, secure interfaces for host interaction:

- **WASI CLI**: Provides command-line arguments and standard input/output streams mapping directly to host terminals.
- **WASI Filesystem**: Exposes highly restricted, pre-opened directories to the guest WASM sandbox, preventing illegal host filesystem traversal.
- **WASI Sockets**: Standardized TCP/UDP network socket connections governed by strict security policies managed by the Crush runtime configuration.
- **WASI HTTP**: Native HTTP client and outbound request APIs mapped directly to host request engines under local permissions.
