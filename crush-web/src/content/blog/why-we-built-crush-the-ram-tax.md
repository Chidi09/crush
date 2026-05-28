---
title: "Why we built Crush: The 1.5 GB RAM tax of local development on Windows"
excerpt: "Local development shouldn't require a virtualized Linux kernel running in the background just to host a Postgres DB and a Redis cache. Here's how we built Crush to run dependencies natively on Windows using OS-level Job Objects—dropping idle RAM consumption from 1.5 GB to under 30 MB."
date: "2026-05-28"
tag: "Announcement"
author: "Chidi"
authorImage: "/founder.jpg"
readingTime: "5 min read"
---

Every Windows developer knows the drill. You boot up your machine, log in, and want to make a quick 2-line code change to a backend service. 

But before you can even run `pnpm dev` or `mvn spring-boot:run`, you have to:
1. Fire up **Docker Desktop**.
2. Wait 45 seconds for the nested **WSL2** utility VM to boot.
3. Spin up your local PostgreSQL and Redis containers.
4. Open Task Manager and watch `vmmem` (WSL2's VM process) immediately gobble up **1.5 GB to 3.0 GB of RAM**—before your actual application has even compiled.

If you are on an 8 GB or 16 GB machine, your browser tabs start reloading, your IDE begins lagging, and your laptop fan begins its high-pitched scream. 

We asked a simple question: **Why are we virtualizing an entire Linux operating system just to run a database and a cache?**

That frustration is why we built **Crush**.

---

## The Virtualization Tax

Docker is a masterpiece of modern software engineering. It solved the "works on my machine" problem for production by packaging applications and their system dependencies into standard OCI containers.

But on Windows, Docker has a fundamental architectural limitation: **containers are a Linux kernel feature (built on namespaces and cgroups).** 

Because Windows runs on the NT kernel, not Linux, Docker Desktop is forced to boot a full virtualized Linux kernel inside a Hyper-V or WSL2 VM just to run your database container. 

This hypervisor layer introduces:
* **Severe Memory Bloat**: The VM has to reserve a large block of RAM to run the guest Linux OS.
* **CPU Tax**: System calls and networking must pass through virtual adapters and VM boundaries, adding micro-delays.
* **File System Sluggishness**: Syncing directories between the Windows host and the Linux VM (via `virtio-FS` or `9p`) is notoriously slow, making hot-restarts of heavy frameworks painful.

---

## Enter Crush: Native Windows Dev Runner

Crush is a tool engineered from the ground up for developers who code on Windows but ship to Linux. 

Instead of virtualizing your stack, **Crush runs your application and its dependencies natively on the Windows host.**

```
Traditional Docker Desktop:
App ──> WSL2 VM (Linux Kernel) ──> Hypervisor ──> Windows Host (NT Kernel)

Crush:
App ──> Windows Host (NT Kernel) [Zero VM overhead]
```

When you type `crush` in your project folder, Crush executes a fast, automated workflow:

1. **Auto-Detects Your Stack**: It parses your codebase to see if you are using Node (Vite, Next, Nuxt, AnalogJS), Java (Spring Boot, Quarkus), Python, Rust, Go, or others.
2. **Starts Native Dependencies**: If you have a `compose.yml` or `application.yml` file, Crush parses it and spins up native Windows binaries of PostgreSQL, MySQL, and Microsoft's **Garnet** (a ultra-fast, Redis-compatible cache engineered in .NET) directly on your host machine.
3. **Optimizes Your Build Loops**: It caches steps. If your `node_modules` or `target/` directories are newer than your package manager lockfiles, it skips the expensive installs and build steps entirely—saving you up to 90 seconds on warm starts.
4. **Enforces Process Teardown**: It binds your entire application and its native databases into a Windows **Job Object**. When you hit `Ctrl+C`, the NT kernel cleanly terminates every single process in the tree instantly. No orphan processes, no "port already in use" errors.

---

## Truthful Performance: Real Numbers

Let's look at a realistic comparison. We took a standard React + Spring Boot + PostgreSQL + Redis project and ran it on a mid-range Windows laptop (Core i7, 16 GB RAM).

| Resource / Action | Docker Desktop (WSL2) | Crush (Native Windows) | The Win |
| :--- | :--- | :--- | :--- |
| **Idle Memory Consumption** | ~1,600 MB (`vmmem`) | **~32 MB** | **50x less RAM** |
| **Warm Start (Launch)** | ~28s (VM Boot + Compose) | **~3s** | **9x faster start** |
| **Process Cleanup** | Manual or slow compose stops | **Sub-millisecond (Ctrl+C)** | **Clean host state** |
| **File System Sync** | Slow cross-OS mounts | **Instant (Native NTFS)** | **Unlocks native SSD speed** |

Crush doesn't magically make your application bundle faster. If your Vite dev server takes 6 seconds to optimize, it will still take 6 seconds. What Crush does is **eliminate the virtualization tax surrounding your code.**

---

## 100% Docker-Compatible

Choosing Crush doesn't mean leaving the Docker ecosystem. We respect Docker's massive role in packaging and deploying software. 

Crush is designed as a **local developer tool, not a production host.**

* It parses your existing `docker-compose.yml` configurations seamlessly.
* It can pull images from any standard OCI registry.
* When you are ready to package your application and deploy it to AWS, Hetzner, or a Linux VPS, simply run:
  ```powershell
  crush eject
  ```
  Crush will automatically generate a perfectly structured `Dockerfile` and `docker-compose.yml` from its stack detection, allowing you to ship standardized OCI images to production.

---

## Try Crush Today

If you are tired of watching Docker Desktop eat your laptop's battery and RAM, give Crush a spin. 

Download our lightweight, single-binary Windows CLI from our [getting started guide](/docs/getting-started) and run `crush` in your project folder today. 

*Welcome to the perfect, VM-free development loop on Windows.*
