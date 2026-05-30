---
title: "Dev-Prod Parity: Designing a Zero-Lock-In Container Eject Workflow"
excerpt: "We believe in complete developer freedom. Learn how Crush's eject command generates standard Dockerfiles and docker-compose files seamlessly."
date: "2026-05-27"
tag: "Design"
author: "Developer Experience Lead"
authorImage: ""
readingTime: "3 min read"
---

Many modern developer tools lock you into their custom ecosystem. We designed **Crush** with a fundamental core philosophy: **Zero Lock-In**.

We believe you should use a tool because it is fast and delighting, not because you are forced to. That is why we built the **Eject Workflow** directly into the core CLI.

## The Developer Dilemma

While Crush's daemonless, zero-config process sandboxing is incredibly fast for local development, you eventually need to deploy your services to production environments running standard container orchestrators like Kubernetes, ECS, or Nomad. 

How do we maintain dev-prod parity without forcing you to write and maintain complex Dockerfiles manually from day one?

## How Eject Works

Whenever you are ready to move a project from Crush to standard Docker runtime environments, simply run:

```bash
crush eject
```

The Crush runtime instantly compiles its internal stack inferences and writes standard, production-ready configurations to your project root:
1. **Dockerfile**: A multi-stage, highly optimized, secure Dockerfile customized exactly for your detected stack (e.g., node, go, rust, python).
2. **docker-compose.yml**: A standard orchestration file configuring environment variables, networking, and dependent services (like database storage or cache layers).

This guarantees **complete mobility**. You can develop locally with sub-second, native speeds inside Crush, and deploy standard OCI images anywhere.
