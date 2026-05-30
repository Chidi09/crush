# Sub-second Quickstart

Get your first container up and running in under 5 minutes with Crush.

## 1. Install Crush
Install the CLI tool directly to your path:

```bash
curl -fsSL https://crushrun.dev/install | sh
```

---

## 2. Enter Your Project Directory
Navigate to any of your active project folders. Crush is highly versatile and intelligently detects runtime stacks automatically.

```bash
cd ~/projects/my-api
```

---

## 3. Run Crush
Launch the runtime. Crush auto-detects Node.js, Python, Go, Rust, Ruby, PHP, and other standard stacks to build perfect sandboxes. It compiles the dependency layer, builds the sandbox, and spawns the container environment immediately:

```bash
~/my-api $ crush
↳ detected: Node.js 20 · TypeScript · Express
↳ deps layer cached (lockfile unchanged)
✓ crushed to image my-api:latest (0.9s · 41 MB)
run it now? [Y/n] Y
✓ running on :3000 — started in 0.3s
```

---

## 4. Hot-Reloading Watcher
To develop interactively with active filesystem watching and hot-reload support, run:

```bash
~/my-api $ crush watch
↳ Active filesystem hot-reload watcher initialized...
↳ Change detected: src/routes/users.ts (modified)
✓ Native container sandbox updated incrementally in 24ms
```

---

## 5. Registry & Deploy Anywhere
Crush compiles projects into standard OCI-compliant formats. You can push your images directly to OCI registries and deploy them cleanly on remote targets:

```bash
# Push the image to a secure registry
~/my-api $ crush push registry.crushrun.dev/myapp:latest
↳ Uploading dependency layer cache [100%]
✓ Image pushed successfully (registry.crushrun.dev/myapp:latest)

# Execute the image deployment on remote targets
~/my-api $ crush run registry.crushrun.dev/myapp:latest --port 3000 -d
↳ Launching daemonless process in remote container scheduler...
✓ Container running in background (PID: 94812) listening on port :3000
```
