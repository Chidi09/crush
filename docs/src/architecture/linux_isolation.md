# Linux Namespace Isolation

On Linux platforms, Crush implements a modern, daemonless container engine that talks directly to native kernel APIs, skipping the heavyweight overhead of traditional daemon architectures like `dockerd`.

---

## Direct Containment via Linux Namespaces

Crush isolates running processes by systematically configuring standard Linux namespace boundaries:

- **PID Namespace (`CLONE_NEWPID`)**: Isolates the process ID space. The containerized application becomes PID 1 inside its own isolated tree, unable to view or interact with processes on the host.
- **NET Namespace (`CLONE_NEWNET`)**: Creates a fully isolated network stack with private interfaces, loopbacks, and separate IP/routing tables.
- **MNT Namespace (`CLONE_NEWNS`)**: Isolates the filesystem mount points. The container gets its own isolated rootfs view, decoupled from host mounts.
- **IPC Namespace (`CLONE_NEWIPC`)**: Segregates System V IPC and POSIX message queues, blocking inter-process communication between host and container processes.
- **UTS Namespace (`CLONE_NEWUTS`)**: Isolates system hostname and NIS domain name configurations.
- **USER Namespace (`CLONE_NEWUSER`)**: Maps user and group IDs (UID/GID) inside the container to unprivileged IDs on the host, preventing host-root privilege escalation.

---

## Resource Controls with cgroups v2

To prevent resource starvation and noisy neighbor syndromes, Crush dynamically constructs unified cgroups v2 slices under `/sys/fs/cgroup/`:

- **Memory Limits**: Implements strict RAM ceilings via `memory.max` and swap boundaries via `memory.swap.max`.
- **CPU Limits**: Restricts CPU consumption by applying active cycle shares through `cpu.max`.
- **I/O Controls**: Throttles block device read/write throughput and IOPS via the `io.max` controller.
- **OOM Kill Handling**: Listens to OOM events on cgroups and triggers graceful/forceful process tree teardowns as configured by Crush policies.
