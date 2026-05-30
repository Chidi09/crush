# OCI Content-Addressed Store

Crush conforms fully to the **OCI (Open Container Initiative) Image Specification**, providing a fast, secure, and highly deduplicated local image registry store.

---

## Content-Addressed Storage (CAS)

All layers, manifests, and configurations inside the Crush local store are stored using **Content-Addressed Storage** based on their cryptographic hashes (SHA-256):

- **Immutable Layering**: Layer blobs are stored by their digest (`sha256:abcd...`), making it mathematically impossible to modify layer contents without invalidating the image digest.
- **Global Deduplication**: If multiple distinct container images share identical baseline dependency or system layers, only a single copy of that layer is downloaded and stored on disk. This saves significant disk space and eliminates redundant network transfers.
- **Zero-Copy Overlays**: Active container runtimes utilize overlay filesystems (`overlay2` on Linux) to compose read-only image layers and a single thin read-write workspace layer on top.

---

## Lazy Image Loading

Traditional engines decompress and unpack complete image tarballs sequentially onto the disk before starting a container. 

Crush accelerates container startup times through **Lazy Loading**:
- **On-Demand Mounting**: It mounts OCI image layers using content-addressed file maps, starting the container *before* the entire image is fully written to disk.
- **Tarball Streaming**: Portions of layers are extracted on-demand from local cache stores, ensuring container runtime boots are completely decoupled from large asset extraction delays.
