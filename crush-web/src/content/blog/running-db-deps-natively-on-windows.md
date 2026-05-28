---
title: "Running PostgreSQL, Redis, and pgvector natively on Windows with zero VM overhead"
excerpt: "A walkthrough of how Crush parses your standard compose.yml, auto-starts native Windows binaries for PostgreSQL and Microsoft's Garnet (a high-performance Redis alternative), and compiles pgvector from source against your host database using MSVC—no containers required."
date: "2026-05-15"
tag: "Deep Dive"
author: "Chidi"
authorImage: "/founder.jpg"
readingTime: "6 min read"
---

When developing modern web applications, your local setup usually includes more than just your frontend and backend code. You need database engines, cache stores, and specialized extensions—like **pgvector** for vector search and AI embeddings.

Under a standard containerized setup, spinning these up means writing a `docker-compose.yml` and running them inside virtualized Linux containers.

On Windows, this introduces a heavy performance penalty. In this deep dive, we’ll explore how **Crush** bypasses virtualization entirely by running your database dependencies natively on Windows with **zero VM overhead**, while maintaining 100% compatibility with your Docker configurations.

---

## 1. Zero-Config Dependency Detection

Crush's philosophy is simple: **we read the files you already have.**

When you run `crush` inside your project folder, it immediately scans your repository for environment configuration files. It parses:
* Standard Docker Compose files (`compose.yml`, `docker-compose.yml`)
* Spring Boot properties (`application.yml`, `application.properties`)
* Environment files (`.env`)

If Crush finds a PostgreSQL or Redis service specified in your Docker Compose file, it doesn't spin up a Docker container. Instead, it **intercepts the configuration** and launches these services natively on your Windows host.

---

## 2. PostgreSQL: Native Host Management

Instead of launching a Postgres container, Crush dynamically interfaces with your native Windows PostgreSQL installation. 

If Postgres is installed on your Windows machine, Crush:
1. **Locates the installation** in your registry or path.
2. **Starts the service** on the configured port (defaulting to `:5432`).
3. **Synchronizes credentials**: It parses the username, password, and database name from your configuration files (e.g. `POSTGRES_DB` and `POSTGRES_PASSWORD` in your compose file) and automatically executes idempotent SQL scripts on your host Postgres to ensure the matching user, password, and database exist.

This means your app connects perfectly to `localhost:5432` using the credentials defined in your codebase, but with the full raw speed of native Windows file systems and network stacks.

---

## 3. Redis-Compat: Microsoft Garnet on Windows

On Linux, starting a Redis container is fast and simple. On Windows, Redis is not natively supported anymore (the official MSOpenTech port was abandoned years ago). Running Redis in Docker Desktop forces you to boot a WSL2 VM.

Crush solves this beautifully by integrating **Microsoft Garnet**.

[Garnet](https://github.com/microsoft/garnet) is a new, state-of-the-art cache store open-sourced by Microsoft Research. Engineered in C#/.NET, it is highly optimized for Windows and is **fully compatible with the Redis serialization protocol (RESP)**.

When Crush detects a `redis` dependency in your project, it starts a lightweight, native instance of Garnet on port `:6379`. 

Because Garnet implements standard Redis commands, your existing Spring, Node, or Python Redis clients (like `ioredis`, `redis-py`, or Spring Data Redis) connect to it seamlessly without changing a single line of code. You get Redis-compatible caching at native speeds, consuming under **15 MB of RAM**.

---

## 4. The Hard Part: Compiling pgvector Natively via MSVC

One of the most complex challenges of native Windows development is using PostgreSQL extensions like **pgvector** (essential for building AI applications with RAG or vector similarity search). 

Usually, developers rely on the official `pgvector/pgvector` Docker image. On Windows, there is no simple installer for pgvector.

Crush handles this automatically via an integrated host compiler pipeline:

1. **Detection**: Crush reads your docker-compose file. If it sees `pgvector/pgvector` as the image name, it notes that the pgvector extension is required.
2. **MSVC Check**: It verifies if you have the **Visual Studio Build Tools** (MSVC compiler) installed on your host. If not, it helpfuly provides the single `winget` command needed to set it up:
   ```powershell
   winget install --id Microsoft.VisualStudio.2022.BuildTools --override "--quiet..."
   ```
3. **Host Compiling**: On the very first run, Crush clones the official `pgvector` repository, locates your native Windows PostgreSQL path, and automatically compiles the extension directly against your host database using MSVC.
4. **Idempotent Install**: It copies the compiled `.dll` and extension control files into your native PostgreSQL directory. Because this requires administrative rights the first time, Crush prompts you to run it from an elevated terminal.
5. **Caching**: Once compiled, Crush marks it as cached. All subsequent runs bypass compiling entirely—starting your vector-enabled database in **under 200 milliseconds**.

---

## Visualizing the Performance Gain

By running PostgreSQL and Garnet (Redis) natively, you bypass the virtual network adapters, memory allocations, and disk translation layers of WSL2.

```
WSL2 / Docker Desktop Layer:
[App] ──(Virtual TCP)──> [WSL2 VM Boundary] ──(ext4 Mount)──> [WSL2 Host Disk]
Result: ~1.2 GB Idle Memory, 10-15% disk overhead

Crush Native Layer:
[App] ──(Local TCP)──> [Native Service] ──(Direct NTFS)──> [Host SSD]
Result: ~45 MB Idle Memory, 0% disk overhead (Raw hardware speed)
```

## Conclusion

Local development should feel fast, light, and friction-free. By shifting from hypervisor-based containers to native Win32 processes, Crush gives you the best of both worlds:
1. **Familiarity**: You write standard Docker Compose configurations.
2. **Performance**: Your hardware runs at its maximum capacity, saving hundreds of megabytes of RAM and achieving sub-second database boots.

Read our [CLI Reference](/docs/cli-reference) to see all the commands available to manage your native services, or download Crush today to try it yourself!
