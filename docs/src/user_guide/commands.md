# CLI Subcommands Reference

This section provides a complete reference guide for the Crush CLI tool and all its subcommands.

---

## 1. Core / Default Command
Auto-detects the project stack, compiles the image, and spawns the container.

**Usage:**
```bash
crush [--platform <platform>]
```

---

## 2. Build
Builds a static, highly optimized Crush image from the active project root directory.

**Usage:**
```bash
crush build [--platform <platform>] [--tag <image-name>:<tag>]
```

---

## 3. Run
Runs an OCI-compliant Crush image in an isolated native container sandbox.

**Usage:**
```bash
crush run <image> [--port <port>] [-d] [--restart <policy>]
```

---

## 4. PS (Process Status)
Lists active running Crush container sandboxes.

**Usage:**
```bash
crush ps [-a]
```

---

## 5. Logs
View and tail real-time output streams from active containers.

**Usage:**
```bash
crush logs <container-id> [-f]
```

---

## 6. Debug
Attach an interactive shell directly to a running container environment.

**Usage:**
```bash
crush debug <container-id>
```

---

## 7. Watch
Launches filesystem hot-reload watcher. Rebuilds and updates sandbox incrementally when file updates are detected.

**Usage:**
```bash
crush watch
```

---

## 8. Compose
Run and orchestrate multi-container apps natively from `docker-compose.yml` declarations.

**Usage:**
```bash
crush compose up [-d]
```

---

## 9. Migrate
Converts standard `Dockerfile` definitions into modern, zero-config `Crushfile` declarations.

**Usage:**
```bash
crush migrate [--dockerfile <path>]
```

---

## 10. Push
Pushes a compiled Crush container image directly to a remote secure OCI registry.

**Usage:**
```bash
crush push <registry>/<repo>:<tag>
```

---

## 11. Pull
Pulls a compiled Crush container image from a remote secure OCI registry.

**Usage:**
```bash
crush pull <registry>/<repo>:<tag>
```

---

## 12. Secrets
Manage encrypted credentials and service tokens.

**Usage:**
```bash
# Set a secret key-value pair
crush secrets set <key> <value>

# List all stored secret names
crush secrets list

# Export secrets to an external store (e.g., Vault)
crush secrets export --to vault
```

---

## 13. Network
Create and manage isolated container network bridges.

**Usage:**
```bash
crush network create <name>
crush network ls
```

---

## 14. Volume
Manage persistent storage volumes for container filesystems.

**Usage:**
```bash
crush volume create <name>
crush volume ls
```

---

## 15. Scan
Scan container images for known security vulnerabilities.

**Usage:**
```bash
crush scan <image>
```

---

## 16. SBOM
Generate a complete Software Bill of Materials (SBOM) for a compiled image.

**Usage:**
```bash
crush sbom <image> [--format spdx-json]
```

---

## 17. System
Inspect system information and run cleaning routines.

**Usage:**
```bash
crush system info
crush system prune
```
