# Windows Job Objects Sandbox

On Windows, Crush eliminates the hypervisor overhead of traditional container engines (which require WSL2 or Hyper-V virtual machines allocating 2GB to 4GB of RAM) by containment natively within the NT kernel using **Windows Job Objects**.

---

## Zero-WSL2 Architecture

Traditional container engines on Windows require running a complete secondary Linux kernel in the background (Hyper-V Moby VM or WSL2). This leads to:
- High CPU/RAM idle consumption (~2.0 GB - 4.0 GB idle).
- Slow file system translations (using 9P mounts).
- Substantial cold-start delay (~15 seconds).

**Crush Native** runs directly inside the NT kernel:
- **No VM Overhead**: Zero Hyper-V or WSL2 VM instances.
- **Minimal Footprint**: Idle memory consumption is ~25 MB.
- **Instant Starts**: Sub-second container environment launches.
- **Native File Access**: Directly accesses NTFS or ReFS drives without protocol translations.

---

## Under the Hood: Windows Containment

To enforce resource constraints and security isolation, Crush interacts with the Windows API directly to set up Job Objects and configure access parameters:

### 1. Job Objects
A **Job Object** allows grouping one or more processes together to be managed as a single unit. Crush configures and locks down Job Objects with strict rules:
- **Memory Caps**: Restricts maximum working set limits (`JobObjectExtendedLimitInformation`).
- **CPU Throttling**: Configures hard CPU cycle utilization percentages via CPU rate limits.
- **Process Spawning**: Prevents escaping processes by disabling the capability to spawn processes outside the job boundary.

### 2. Network Isolation
Network ports are mapped through the native Windows Filtering Platform (WFP) or bound directly using local socket proxies, ensuring clean port forwarding and host loopback access.

### 3. File System Layering
Process workspaces are isolated using NTFS hardlinks, junction points, or symbolic links inside an isolated workspace directory, maintaining distinct overlays without full file duplication.
