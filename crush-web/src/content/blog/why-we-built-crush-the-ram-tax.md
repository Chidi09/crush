---
title: "Why We Built Crush: The RAM Tax of Modern Container Desktops"
excerpt: "Traditional local container engines demand gigabytes of idle memory. Here is how Crush eliminates the RAM tax entirely."
date: "2026-05-30"
tag: "Engineering"
author: "Crush Core Team"
authorImage: ""
readingTime: "4 min read"
---

Every modern software engineer uses containers. We build them, run them, and deploy them. But on our local development machines, modern container desktops charge a massive **RAM Tax**.

## The Problem: The Guest Kernel Tax

If you run Docker Desktop or similar systems on macOS or Windows, a virtual machine must boot in the background (Hyper-V, WSL2, or QEMU guest kernel). Even when no containers are active, this background VM claims **2GB to 4GB of RAM** immediately.

If you are developing on a standard laptop with 16GB of memory, this RAM tax consumes up to a quarter of your entire system's capacity just to sit idle. Furthermore, crossing the virtual machine boundary introduces file system translation lag (such as 9P mounts or virtiofs sync bottlenecks), making operations like active code reloading sluggish.

## The Crush Solution: Native Isolation

We built **Crush** because we wanted a modern container runtime that runs containers natively on host operating systems without virtualization.

By executing container processes directly on the host OS kernel:
- **No VM Overhead**: Crush utilizes OS process-containment APIs directly (Linux Namespaces / cgroups v2, and Windows Job Objects).
- **Minimal Memory Footprint**: Idle RAM consumption is reduced to just **~25 MB** (a 99% savings compared to standard VMs).
- **Ultra-Fast Startup**: Process containers start in **sub-second** cold starts since they do not need to boot a virtual kernel.
- **Native File Performance**: Workspace folders are mounted directly, unlocking native storage read/write performance.

## A Balanced Engineering Choice

We believe containers should be lightweight and immediate. Crush returns power to the developer by returning resources directly back to your local development machine.
