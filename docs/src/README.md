# Crush Runtime

Welcome to the documentation for **Crush**: a from-scratch, production-grade container runtime written entirely in Rust.

## What is Crush?

Crush is a daemonless, native container runtime that offers:
- **Windows-native isolation** using Job Objects instead of heavy virtualization VMs.
- **Lightweight Firecracker microVMs** for running Linux containers on Windows.
- **Sub-second cold starts** through content-addressed layers and lazy image loading.
- **Intelligent error diagnosis** intercepting compiler/runtime crashes and providing Anthropic Claude auto-fixes.
- **Zero-config builds** with automated manifest language fingerprinting.
- **First-class WASM execution** linking wasmtime and WASI preview 2 natively.

## Getting Started

To get started with Crush, explore the following pages:
- [Installation](user_guide/installation.md)
- [Quickstart Guide](user_guide/quickstart.md)
- [CLI Reference](user_guide/commands.md)
- [Architecture Overview](architecture/README.md)
