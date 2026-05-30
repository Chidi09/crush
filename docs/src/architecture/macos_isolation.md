# macOS Apple Virtualization

Unlike Linux and Windows, which support native process containment directly inside their host OS kernels, macOS does not provide native containment features like namespaces or Job Objects for running Linux container binaries.

To deliver sub-second container startup times on macOS, Crush leverages Apple's native systems virtualization technologies directly.

---

## Direct Hypervisor Integration

Instead of wrapping a third-party virtualization engine (like QEMU or VirtualBox) or managing a background daemon machine like Docker Desktop does, Crush interfaces directly with:

1. **Hypervisor.framework**: Apple's native, low-level user-space hypervisor API that allows managing CPUs and guest physical memory spaces directly on Intel and Apple Silicon chips.
2. **Virtualization.framework**: A high-level API introduced in macOS Big Sur that provides built-in support for booting lightweight Linux guest kernels, configuring VirtIO storage devices, and setting up network sockets natively.

---

## Crush Apple MicroVMs

When executing a Linux container on macOS, Crush:
- **Instant Bootstrapping**: Spins up a highly optimized Apple MicroVM running a customized, ultra-minimal Linux kernel in less than **400 milliseconds**.
- **Shared Memory Cache**: Employs direct memory-mapped virtio file systems to share files between the host macOS system and the guest microVM, avoiding the significant I/O latency of traditional network file mounts.
- **Dynamic Slicing**: Dynamically allocates CPU and RAM limits to the microVM based on active container requirements, returning idle resources to the macOS host system automatically.
