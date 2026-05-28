---
title: "Teardown guarantees on Windows: How Job Objects prevent orphan process leaks"
excerpt: "Ever hit Ctrl+C on your terminal only to find out the port is still in use because a child node or java process didn't die? Learn how Crush utilizes the Windows NT Kernel's Job Objects to enforce a strict boundary, ensuring every child process in the tree is instantly and cleanly reaped on exit."
date: "2026-04-29"
tag: "Technical"
author: "Chidi"
authorImage: "/founder.jpg"
readingTime: "5 min read"
---

It’s a scenario every web developer has encountered. 

You’re running a dev server (like a Vite, Next.js, or Spring Boot application) in your terminal. You make some heavy configuration changes and want to reboot it. You hit `Ctrl+C` in your terminal shell, the prompt returns, and you type `npm run dev` again.

Then, you're greeted with this dreaded error message:
```
Error: listen EADDRINUSE: address already in use :::3000
```

You open your browser, and sure enough, the old page is still serving. You look at your terminal; the process supposedly exited. Now you have to open Task Manager, hunt down the rogue `node.exe` or `java.exe` process that was left behind as an orphan, force-kill it, and restart.

This happens because of a fundamental issue in process-tree lifecycle management. 

In this article, we’ll dive into why this happens on Windows and how **Crush** utilizes a powerful NT kernel feature—**Job Objects**—to guarantee complete, sub-millisecond cleanup of your entire development environment.

---

## Why Child Processes Leak

When you run a parent process in a terminal (like `npm` or `mvn`), that process often spawns multiple child processes. For example:
* `npm` spawns `node` to run your dev server.
* `pnpm` in a monorepo might spawn several `node` microservice compilers.
* A Python script might spawn multiple background worker tasks.

When you hit `Ctrl+C`, a signal is sent to the parent process. If the parent process exits abruptly, or if it doesn't cleanly propagate the termination signal to all its children, those child processes become **orphans**. 

Because they are no longer linked to an active console session, they stay alive in the background, quietly holding onto memory and—worst of all—**binding network ports**.

On Linux, developers sometimes use process groups (`kill -9 -<PGID>`) or systemd slices to handle this. On Windows, managing child process trees natively has traditionally been a major pain point.

---

## Enter Win32 Job Objects

To solve this problem cleanly on Windows, Crush leverages a first-class operating system primitive: **Job Objects**.

A Job Object is an NT kernel feature that allows you to group one or more processes together as a single unit. Once processes are inside a Job Object, the operating system can apply strict, collective resource limits and lifecycle rules to all of them.

```
[Crush CLI] ──> Creates Job Object
                     │
     ┌───────────────┼───────────────┐
     ▼               ▼               ▼
[Postgres]     [Garnet/Redis]   [Vite App]   (All processes bound to Job)
```

When Crush starts your application and its native database dependencies, it executes the following systems-level calls:

1. **Creates a Job Object**: It calls the Win32 API `CreateJobObjectW`.
2. **Configures Teardown Limits**: It applies the `JOBOBJECT_EXTENDED_LIMIT_INFORMATION` structure to the job, specifically setting the flag:
   ```
   JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
   ```
   This flag tells the Windows kernel: **"If the handle to this Job Object is closed (or if the parent process exits), immediately terminate every process associated with this job."**
3. **Spawns and Binds Processes**: Crush launches PostgreSQL, Garnet/Redis, and your main application process, and immediately assigns them to the Job Object via `AssignProcessToJobObject`.

---

## Why This Guarantee is Flawless

Because the process teardown is managed directly by the **NT kernel**, it is completely independent of your application's code. 

It doesn't matter if your Vite app hangs, if Spring Boot is in the middle of a heavy database migration, or if your package manager crashes. The moment the Crush CLI process exits (whether via `Ctrl+C`, a terminal crash, or an unexpected shutdown), the kernel automatically closes the Job Object handle.

Instantly, the kernel sweeps through the active process table and reaps every single process assigned to that job in **less than 1 millisecond**.

No orphan processes. No locked ports. No Task Manager hunts.

---

## Custom Resource Caps

Besides guaranteed cleanup, binding your dev stack to a Job Object unlocks another major feature: **native resource capping**.

If you are developing a resource-heavy application, you might want to prevent it from consuming all your system's RAM or CPU. Inside your `.crush.toml` or via CLI flags, you can specify limits:

```powershell
crush --memory 512MB --cpus 2
```

Crush translates these directly into kernel limits on your Job Object (`JobObjectBasicLimitInformation`). Windows will enforce these caps natively at the kernel scheduler level, ensuring your dev stack behaves exactly like a container, but with **zero hypervisor overhead**.

---

## Conclusion

Windows is an incredibly capable developer platform, but developers have often been pushed toward VMs (like WSL2) simply because of process management and dependency issues. 

By leveraging native Win32 Job Objects, Crush provides container-like cleanup and limits with the speed and efficiency of direct host execution.

Check out our [The Crushfile Specification](/docs/crushfile) guide to see how you can configure Job Object caps for your projects!
