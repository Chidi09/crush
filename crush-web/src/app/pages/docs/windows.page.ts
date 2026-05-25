import { Component, OnInit } from '@angular/core';
import { Title, Meta } from '@angular/platform-browser';
import { RouterLink } from '@angular/router';
import { HlmIconComponent } from '@spartan-ng/ui-icon-helm';
import { DocsSidebarComponent } from '../../components/docs-sidebar/docs-sidebar.component';

@Component({
  selector: 'page-windows',
  standalone: true,
  imports: [RouterLink, HlmIconComponent, DocsSidebarComponent],
  template: `
    <div class="mx-auto max-w-7xl px-4 py-16 sm:px-6 lg:px-8">
      <div class="flex flex-col md:flex-row gap-12">
        <app-docs-sidebar />
        <article class="flex-1 min-w-0">
          <!-- Page Header -->
          <div class="border-b border-crush-border/30 pb-6 mb-10 select-none">
            <span class="text-xs font-bold uppercase tracking-wider text-crush-orange"
              >Windows Internals</span
            >
            <h1 class="text-3xl font-extrabold text-white tracking-tight mt-1 mb-2">
              Windows Developer Guide
            </h1>
            <p class="text-base text-crush-textMuted">
              Explore native containment with Job Objects, Firecracker microVM cross-compilation,
              and deep systems engineering.
            </p>
          </div>

          <!-- Section 1: Zero-WSL2 Architecture -->
          <section class="mb-14">
            <h2 class="text-xl font-bold text-white mb-4 select-none">Zero-WSL2 Architecture</h2>
            <p class="text-sm text-crush-textMuted leading-relaxed mb-6">
              Traditional container engines on Windows require virtualizing a complete secondary
              operating system kernel in the background (WSL2 or Hyper-V Moby VM). This incurs high
              hypervisor overhead, allocates an idle 2GB–4GB memory slice, and enforces slow 9P file
              system mounts.
            </p>
            <p class="text-sm text-crush-textMuted leading-relaxed mb-8">
              <span class="text-crush-orange font-semibold">Crush</span> eliminates hypervisor
              latency by employing **Windows Job Objects** and process-isolation boundary APIs
              directly inside the native NT kernel.
            </p>

            <!-- Comparison Graphic -->
            <div class="grid gap-6 md:grid-cols-2 mb-8 select-none">
              <!-- Docker Desktop WSL2 Card -->
              <div
                class="rounded-xl border border-crush-border/40 bg-crush-black/40 p-5 hover:border-crush-border/75 transition-all duration-300"
              >
                <div class="flex items-center gap-2 mb-3">
                  <span class="w-2.5 h-2.5 rounded-full bg-red-500"></span>
                  <h3 class="text-xs font-bold text-crush-textMuted uppercase font-mono">
                    Docker Desktop (WSL2)
                  </h3>
                </div>
                <div class="space-y-2.5 text-xs text-crush-textMuted font-mono">
                  <div class="flex justify-between border-b border-crush-border/20 pb-1.5">
                    <span>Virtual Machine</span>
                    <span class="text-red-400">Yes (Hyper-V/WSL)</span>
                  </div>
                  <div class="flex justify-between border-b border-crush-border/20 pb-1.5">
                    <span>Memory Idle</span>
                    <span class="text-red-400">~2.0 GB - 4.0 GB</span>
                  </div>
                  <div class="flex justify-between border-b border-crush-border/20 pb-1.5">
                    <span>Startup Time</span>
                    <span class="text-red-400">~15.0 seconds</span>
                  </div>
                  <div class="flex justify-between border-b border-crush-border/20 pb-1.5">
                    <span>File System Translation</span>
                    <span class="text-red-400">9P / virtio-fs Sync Lag</span>
                  </div>
                  <div class="flex justify-between">
                    <span>Process Overhead</span>
                    <span class="text-red-400">Double Scheduling</span>
                  </div>
                </div>
              </div>

              <!-- Crush Native Card -->
              <div
                class="rounded-xl border border-crush-orange/20 bg-gradient-to-b from-crush-orange/5 to-transparent p-5 hover:border-crush-orange/40 transition-all duration-300"
              >
                <div class="flex items-center gap-2 mb-3">
                  <span class="w-2.5 h-2.5 rounded-full bg-emerald-500 animate-pulse-glow"></span>
                  <h3 class="text-xs font-bold text-crush-orangeLight uppercase font-mono">
                    Crush Native (Win32)
                  </h3>
                </div>
                <div class="space-y-2.5 text-xs text-crush-textMuted font-mono">
                  <div class="flex justify-between border-b border-crush-border/20 pb-1.5">
                    <span>Virtual Machine</span>
                    <span class="text-emerald-400 font-bold">No (Zero VM)</span>
                  </div>
                  <div class="flex justify-between border-b border-crush-border/20 pb-1.5">
                    <span>Memory Idle</span>
                    <span class="text-emerald-400 font-bold">~25 MB</span>
                  </div>
                  <div class="flex justify-between border-b border-crush-border/20 pb-1.5">
                    <span>Startup Time</span>
                    <span class="text-emerald-400 font-bold">~0.3 seconds</span>
                  </div>
                  <div class="flex justify-between border-b border-crush-border/20 pb-1.5">
                    <span>File System Translation</span>
                    <span class="text-emerald-400 font-bold">Direct NTFS Handles</span>
                  </div>
                  <div class="flex justify-between">
                    <span>Process Overhead</span>
                    <span class="text-emerald-400 font-bold">Native NT Scheduler</span>
                  </div>
                </div>
              </div>
            </div>

            <!-- Deep Dive Alert Box -->
            <div
              class="flex gap-4 p-5 rounded-xl border border-crush-orange/15 bg-crush-orange/5 mb-6 text-xs sm:text-sm"
            >
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                class="h-6 w-6 text-crush-orange shrink-0 mt-0.5 select-none"
              >
                <path d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
              <div>
                <p class="font-bold text-crush-orangeLight mb-1.5">
                  Under-the-Hood: Win32 Kernel APIs
                </p>
                <p class="text-crush-textMuted leading-relaxed">
                  Crush leverages standard Win32 kernel API calls directly. When you instantiate a
                  sandbox, Crush invokes <code>CreateJobObjectW</code>, configures memory and thread
                  boundaries using <code>SetInformationJobObject</code> (with flags like
                  <code>JOB_OBJECT_LIMIT_WORKING_SET_SIZE</code> and
                  <code>JobObjectCpuRateControlInformation</code>), restricts nesting limits via
                  <code>ActiveProcessLimit</code>, and assigns recursively grouped processes cleanly
                  with <code>AssignProcessToJobObject</code>.
                </p>
              </div>
            </div>
          </section>

          <!-- Section 2: Step-by-Step Local Deployment Example -->
          <section class="mb-14">
            <h2 class="text-xl font-bold text-white mb-4 select-none">
              Interactive Step-by-Step Local Walkthrough
            </h2>
            <p class="text-sm text-crush-textMuted leading-relaxed mb-6">
              Learn how to initialize, configure, build, and run a micro-service natively on
              Windows. These multi-line mock consoles represent complete developer cycles:
            </p>

            <!-- Step A Console -->
            <div class="space-y-6">
              <div>
                <div class="flex items-center gap-2 mb-2 select-none">
                  <span
                    class="flex h-5 w-5 items-center justify-center rounded-full bg-crush-orange/10 text-crush-orange text-[10px] font-bold border border-crush-orange/20"
                    >A</span
                  >
                  <span class="text-xs font-bold text-white uppercase tracking-wider font-mono"
                    >Initialize Project Sandbox</span
                  >
                </div>
                <div
                  class="rounded-xl border border-crush-border/40 bg-crush-black/85 overflow-hidden"
                >
                  <div
                    class="flex items-center justify-between px-4 py-2.5 border-b border-crush-border/30 bg-crush-surface/30 select-none"
                  >
                    <div class="flex items-center gap-1.5">
                      <span class="w-2.5 h-2.5 rounded-full bg-[#ff5f56]"></span>
                      <span class="w-2.5 h-2.5 rounded-full bg-[#ffbd2e]"></span>
                      <span class="w-2.5 h-2.5 rounded-full bg-[#27c93f]"></span>
                      <span class="text-[10px] text-crush-textMuted font-mono ml-2"
                        >Developer PowerShell</span
                      >
                    </div>
                    <span class="text-[9px] text-crush-textMuted uppercase font-semibold"
                      >init</span
                    >
                  </div>
                  <div
                    class="p-5 font-mono text-xs sm:text-sm text-crush-text leading-relaxed whitespace-pre overflow-x-auto"
                  >
                    <span class="text-crush-textMuted">PS C:\\projects\\my-service> </span
                    ><span class="text-crush-orange font-bold">crush</span> init
                    <span class="text-crush-textMuted">↳ Scanning files...</span>
                    <span class="text-crush-textMuted"
                      >↳ Found package.json -> Node.js Express API detected</span
                    >
                    <span class="text-emerald-400"
                      >✓ Generated default configuration in .\\Crushfile</span
                    >
                    <span class="text-emerald-400">✓ Isolated local layer cache prepared</span>
                  </div>
                </div>
              </div>

              <!-- Step B Console -->
              <div>
                <div class="flex items-center gap-2 mb-2 select-none">
                  <span
                    class="flex h-5 w-5 items-center justify-center rounded-full bg-crush-orange/10 text-crush-orange text-[10px] font-bold border border-crush-orange/20"
                    >B</span
                  >
                  <span class="text-xs font-bold text-white uppercase tracking-wider font-mono"
                    >Configure the Crushfile</span
                  >
                </div>
                <p class="text-xs text-crush-textMuted mb-2 leading-relaxed">
                  Your local <code>Crushfile</code> controls execution parameters natively. Below is
                  a standard Windows port and memory limit setup:
                </p>
                <div
                  class="rounded-xl border border-crush-border/40 bg-crush-black/85 overflow-hidden"
                >
                  <div
                    class="flex items-center justify-between px-4 py-2.5 border-b border-crush-border/30 bg-crush-surface/30 select-none"
                  >
                    <div class="flex items-center gap-1.5">
                      <span class="w-2.5 h-2.5 rounded-full bg-[#ff5f56]"></span>
                      <span class="w-2.5 h-2.5 rounded-full bg-[#ffbd2e]"></span>
                      <span class="w-2.5 h-2.5 rounded-full bg-[#27c93f]"></span>
                      <span class="text-[10px] text-crush-textMuted font-mono ml-2"
                        >Crushfile Configuration</span
                      >
                    </div>
                    <span class="text-[9px] text-crush-textMuted uppercase font-semibold"
                      >toml</span
                    >
                  </div>
                  <div
                    class="p-5 font-mono text-xs sm:text-sm text-crush-text leading-relaxed whitespace-pre overflow-x-auto"
                  >
                    <span class="text-amber-400 font-bold">[container]</span>
                    <span class="text-crush-textMuted">name = "my-service"</span>
                    <span class="text-crush-textMuted">version = "1.0.0"</span>

                    <span class="text-amber-400 font-bold">[resources]</span>
                    <span class="text-crush-textMuted">memory_limit_mb = 128</span>
                    <span class="text-crush-textMuted">cpu_shares = 512</span>

                    <span class="text-amber-400 font-bold">[ports]</span>
                    <span class="text-crush-textMuted">expose = [5000]</span>
                  </div>
                </div>
              </div>

              <!-- Step C Console -->
              <div>
                <div class="flex items-center gap-2 mb-2 select-none">
                  <span
                    class="flex h-5 w-5 items-center justify-center rounded-full bg-crush-orange/10 text-crush-orange text-[10px] font-bold border border-crush-orange/20"
                    >C</span
                  >
                  <span class="text-xs font-bold text-white uppercase tracking-wider font-mono"
                    >Build and Pack Native Image</span
                  >
                </div>
                <div
                  class="rounded-xl border border-crush-border/40 bg-crush-black/85 overflow-hidden"
                >
                  <div
                    class="flex items-center justify-between px-4 py-2.5 border-b border-crush-border/30 bg-crush-surface/30 select-none"
                  >
                    <div class="flex items-center gap-1.5">
                      <span class="w-2.5 h-2.5 rounded-full bg-[#ff5f56]"></span>
                      <span class="w-2.5 h-2.5 rounded-full bg-[#ffbd2e]"></span>
                      <span class="w-2.5 h-2.5 rounded-full bg-[#27c93f]"></span>
                      <span class="text-[10px] text-crush-textMuted font-mono ml-2"
                        >Developer PowerShell</span
                      >
                    </div>
                    <span class="text-[9px] text-crush-textMuted uppercase font-semibold"
                      >build</span
                    >
                  </div>
                  <div
                    class="p-5 font-mono text-xs sm:text-sm text-crush-text leading-relaxed whitespace-pre overflow-x-auto"
                  >
                    <span class="text-crush-textMuted">PS C:\\projects\\my-service> </span
                    ><span class="text-crush-orange font-bold">crush</span> build
                    <span class="text-crush-textMuted">↳ Validating configuration limits...</span>
                    <span class="text-crush-textMuted"
                      >↳ Bundling system dependencies... (cached)</span
                    >
                    <span class="text-crush-textMuted"
                      >↳ Compiling native Win32 layer manifests...</span
                    >
                    <span class="text-emerald-400"
                      >✓ OCI win32 container built: my-service:1.0.0 (0.28s)</span
                    >
                  </div>
                </div>
              </div>

              <!-- Step D Console -->
              <div>
                <div class="flex items-center gap-2 mb-2 select-none">
                  <span
                    class="flex h-5 w-5 items-center justify-center rounded-full bg-crush-orange/10 text-crush-orange text-[10px] font-bold border border-crush-orange/20"
                    >D</span
                  >
                  <span class="text-xs font-bold text-white uppercase tracking-wider font-mono"
                    >Execute Container Sandbox Natively</span
                  >
                </div>
                <div
                  class="rounded-xl border border-crush-border/40 bg-crush-black/85 overflow-hidden"
                >
                  <div
                    class="flex items-center justify-between px-4 py-2.5 border-b border-crush-border/30 bg-crush-surface/30 select-none"
                  >
                    <div class="flex items-center gap-1.5">
                      <span class="w-2.5 h-2.5 rounded-full bg-[#ff5f56]"></span>
                      <span class="w-2.5 h-2.5 rounded-full bg-[#ffbd2e]"></span>
                      <span class="w-2.5 h-2.5 rounded-full bg-[#27c93f]"></span>
                      <span class="text-[10px] text-crush-textMuted font-mono ml-2"
                        >Developer PowerShell</span
                      >
                    </div>
                    <span class="text-[9px] text-crush-textMuted uppercase font-semibold">run</span>
                  </div>
                  <div
                    class="p-5 font-mono text-xs sm:text-sm text-crush-text leading-relaxed whitespace-pre overflow-x-auto"
                  >
                    <span class="text-crush-textMuted">PS C:\\projects\\my-service> </span
                    ><span class="text-crush-orange font-bold">crush</span> run my-service:1.0.0
                    --port 5000
                    <span class="text-crush-textMuted"
                      >↳ Allocating Win32 Kernel Job boundaries (Job ID: job-48192)</span
                    >
                    <span class="text-crush-textMuted"
                      >↳ Routing WinSock socket handle directly to port :5000</span
                    >
                    <span class="text-crush-textMuted"
                      >↳ Spawning child process natively inside sandbox (PID: 18274)</span
                    >
                    <span class="text-emerald-400 font-bold"
                      >✓ Container status: running (startup latency: 38ms)</span
                    >
                    <span class="text-emerald-400 font-bold"
                      >✓ Listening at http://localhost:5000</span
                    >
                  </div>
                </div>
              </div>
            </div>
          </section>

          <!-- Section 3: Linux Compiles on Windows -->
          <section class="mb-14">
            <h2 class="text-xl font-bold text-white mb-4 select-none">
              Cross-Platform Compiles (Linux Targets on Windows)
            </h2>
            <p class="text-sm text-crush-textMuted leading-relaxed mb-4">
              If Crush runs containers as native Windows processes, how does it build Linux
              OCI-compliant container images?
            </p>
            <p class="text-sm text-crush-textMuted leading-relaxed mb-6">
              When you execute
              <code
                ><span class="text-crush-orange font-bold">crush</span> build --platform
                linux/amd64</code
              >, Crush automatically instantiates an isolated **Firecracker microVM** builder
              sandbox on your host. Booting in under **150ms**, the builder maps your host workspace
              directly via highly optimized <code>virtio-fs</code> memory bridges. The guest Linux
              kernel compiles the dependencies natively, outputs standard ELF layers, registers the
              OCI manifest, and terminates instantly.
            </p>

            <!-- Beautiful Pipeline diagram -->
            <div
              class="rounded-xl border border-crush-orange/20 bg-gradient-to-b from-crush-orange/5 to-transparent p-6 mb-6 select-none"
            >
              <p
                class="text-crush-orange font-bold text-xs uppercase tracking-wider mb-4 font-mono text-center"
              >
                Linux Target Compile Flow
              </p>
              <div
                class="flex flex-col sm:flex-row items-center justify-between gap-4 text-xs font-mono"
              >
                <div
                  class="flex items-center gap-2 bg-crush-surface/60 border border-crush-border/60 px-3.5 py-2 rounded-lg text-white"
                >
                  <span>NTFS Host Directory</span>
                </div>

                <svg
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2.5"
                  class="h-4 w-4 text-crush-orange rotate-90 sm:rotate-0"
                >
                  <line x1="5" y1="12" x2="19" y2="12" />
                  <polyline points="12 5 19 12 12 19" />
                </svg>

                <div
                  class="flex items-center gap-2 bg-crush-orange/10 border border-crush-orange/30 px-3.5 py-2 rounded-lg text-crush-orangeLight"
                >
                  <span>Virtio-FS Memory Map</span>
                </div>

                <svg
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2.5"
                  class="h-4 w-4 text-crush-orange rotate-90 sm:rotate-0"
                >
                  <line x1="5" y1="12" x2="19" y2="12" />
                  <polyline points="12 5 19 12 12 19" />
                </svg>

                <div
                  class="flex items-center gap-2 bg-crush-surface/60 border border-crush-border/60 px-3.5 py-2 rounded-lg text-white font-bold"
                >
                  <span>Firecracker VM (150ms)</span>
                </div>

                <svg
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2.5"
                  class="h-4 w-4 text-crush-orange rotate-90 sm:rotate-0"
                >
                  <line x1="5" y1="12" x2="19" y2="12" />
                  <polyline points="12 5 19 12 12 19" />
                </svg>

                <div
                  class="flex items-center gap-2 bg-emerald-500/10 border border-emerald-500/30 px-3.5 py-2 rounded-lg text-emerald-400"
                >
                  <span>Linux OCI Image</span>
                </div>
              </div>
            </div>

            <div
              class="flex gap-3.5 p-4 rounded-xl border border-emerald-500/15 bg-emerald-500/5 text-xs sm:text-sm select-none"
            >
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                class="h-5 w-5 text-emerald-400 shrink-0 mt-0.5"
              >
                <path d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
              <div>
                <p class="font-bold text-emerald-400 mb-0.5">High Frequency Hot-Reloading Sync</p>
                <p class="text-crush-textMuted leading-relaxed">
                  Virtio-FS synchronizations enable Crush to share direct kernel-mapped pages.
                  Incremental compilations are instantaneous; file writes triggered in VS Code
                  inside Windows immediately initiate lightning-fast hot-reloads within the builder
                  environment.
                </p>
              </div>
            </div>
          </section>

          <!-- Section 4: Network Namespaces and Routing -->
          <section class="mb-14">
            <h2 class="text-xl font-bold text-white mb-4 select-none">
              Socket-Level Port Allocation
            </h2>
            <p class="text-sm text-crush-textMuted leading-relaxed mb-4">
              Traditional WSL2 containers must route network flows via complex Hyper-V virtual
              network switches, routing bridges, and NAT mapping utilities (e.g.,
              <code>wslhost.exe</code>). This translation degrades performance and causes conflicts
              with host interface listeners.
            </p>
            <p class="text-sm text-crush-textMuted leading-relaxed">
              <span class="text-crush-orange font-semibold">Crush</span> routes network bindings
              directly at the WinSock socket layer. Port bindings map straight to host loopback
              interfaces natively. There are no virtual routers, internal switches, or translation
              latency: your application processes network packets at host wire-speed.
            </p>
          </section>

          <!-- Section 5: Troubleshooting & System Settings -->
          <section class="mb-14">
            <h2 class="text-xl font-bold text-white mb-4 select-none">
              Windows Tunables & Troubleshooting
            </h2>

            <div class="space-y-4">
              <!-- Antivirus -->
              <div class="rounded-xl border border-crush-border/40 bg-crush-surface/10 p-5">
                <h3 class="text-sm font-bold text-white mb-2">
                  Windows Defender/Antivirus Latency
                </h3>
                <p class="text-xs text-crush-textMuted leading-relaxed">
                  Real-time filesystem scanning can inspect newly created image layer hierarchies,
                  which adds latency to process spawns. To achieve maximum sub-second compilation
                  speeds, we recommend excluding the following directories in Windows Security:
                  <code
                    class="block mt-2 px-3 py-1.5 bg-crush-black/50 rounded font-mono text-[10px] text-crush-text border border-crush-border/30"
                    >C:\\ProgramData\\crush\\*<br />%USERPROFILE%\\.crush\\*</code
                  >
                </p>
              </div>

              <!-- Symbolic link privileges -->
              <div class="rounded-xl border border-crush-border/40 bg-crush-surface/10 p-5">
                <h3 class="text-sm font-bold text-white mb-2">NTFS Symbolic Links & Permissions</h3>
                <p class="text-xs text-crush-textMuted leading-relaxed">
                  Crush leverages NTFS symbolic links and junction trees to isolate node_modules and
                  system libraries. By default, Windows restricts symlink creation to elevated
                  sessions. Enable **Windows Developer Mode** in your system settings (Update &
                  Security -> For developers) to permit link creation without launching shells in
                  administrator mode (<code>SeCreateSymbolicLinkPrivilege</code>).
                </p>
              </div>
            </div>
          </section>

          <!-- Section 6: Windows Package Installation -->
          <section class="mb-14">
            <h2 class="text-xl font-bold text-white mb-4 select-none">
              Package Manager Installations
            </h2>
            <p class="text-sm text-crush-textMuted mb-6 leading-relaxed">
              Install the lightweight binary suite using your preferred Windows package management
              system without third-party hypervisor components:
            </p>

            <div class="space-y-6">
              <!-- PowerShell Install -->
              <div>
                <div
                  class="flex items-center justify-between px-4 py-2 border border-b-0 border-crush-border/40 rounded-t-xl bg-crush-surface/30 select-none"
                >
                  <div class="flex items-center gap-1.5">
                    <span class="w-2 h-2 rounded-full bg-[#ff5f56]"></span>
                    <span class="w-2 h-2 rounded-full bg-[#ffbd2e]"></span>
                    <span class="w-2 h-2 rounded-full bg-[#27c93f]"></span>
                    <span class="text-[10px] text-crush-textMuted font-mono ml-2"
                      >PowerShell Inline Installer</span
                    >
                  </div>
                  <span class="text-[9px] text-crush-textMuted uppercase font-semibold">pwsh</span>
                </div>
                <div
                  class="rounded-b-xl border border-crush-border/40 bg-crush-black/85 p-4 font-mono text-xs sm:text-sm text-crush-text overflow-x-auto"
                >
                  <code>irm https://crushrun.dev/install.ps1 | iex</code>
                </div>
              </div>

              <!-- Winget Install -->
              <div>
                <div
                  class="flex items-center justify-between px-4 py-2 border border-b-0 border-crush-border/40 rounded-t-xl bg-crush-surface/30 select-none"
                >
                  <div class="flex items-center gap-1.5">
                    <span class="w-2 h-2 rounded-full bg-[#ff5f56]"></span>
                    <span class="w-2 h-2 rounded-full bg-[#ffbd2e]"></span>
                    <span class="w-2 h-2 rounded-full bg-[#27c93f]"></span>
                    <span class="text-[10px] text-crush-textMuted font-mono ml-2"
                      >Winget Package Manager</span
                    >
                  </div>
                  <span class="text-[9px] text-crush-textMuted uppercase font-semibold"
                    >winget</span
                  >
                </div>
                <div
                  class="rounded-b-xl border border-crush-border/40 bg-crush-black/85 p-4 font-mono text-xs sm:text-sm text-crush-text overflow-x-auto"
                >
                  <code
                    >winget install
                    <span class="text-crush-orange font-bold">Crush</span>.Container</code
                  >
                </div>
              </div>

              <!-- Scoop Install -->
              <div>
                <div
                  class="flex items-center justify-between px-4 py-2 border border-b-0 border-crush-border/40 rounded-t-xl bg-crush-surface/30 select-none"
                >
                  <div class="flex items-center gap-1.5">
                    <span class="w-2 h-2 rounded-full bg-[#ff5f56]"></span>
                    <span class="w-2 h-2 rounded-full bg-[#ffbd2e]"></span>
                    <span class="w-2 h-2 rounded-full bg-[#27c93f]"></span>
                    <span class="text-[10px] text-crush-textMuted font-mono ml-2"
                      >Scoop Package Manager</span
                    >
                  </div>
                  <span class="text-[9px] text-crush-textMuted uppercase font-semibold">scoop</span>
                </div>
                <div
                  class="rounded-b-xl border border-crush-border/40 bg-crush-black/85 p-4 font-mono text-xs sm:text-sm text-crush-text leading-relaxed overflow-x-auto"
                >
                  <code
                    >scoop bucket add
                    <span class="text-crush-orange font-bold">crush</span>
                    https://github.com/crushcontainer/scoop-bucket<br />scoop install
                    <span class="text-crush-orange font-bold">crush</span></code
                  >
                </div>
              </div>
            </div>
          </section>

          <!-- Footer Navigation Links -->
          <div
            class="flex items-center justify-between border-t border-crush-border/30 pt-8 mt-16 select-none"
          >
            <a
              routerLink="/docs/docker-migration"
              class="inline-flex items-center gap-2 text-sm text-crush-textMuted hover:text-white transition-colors"
            >
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                class="h-4 w-4"
              >
                <line x1="19" y1="12" x2="5" y2="12" />
                <polyline points="12 19 5 12 12 5" />
              </svg>
              Docker Migration
            </a>
            <a
              routerLink="/docs/security"
              class="inline-flex items-center gap-2 text-sm text-crush-orange hover:text-crush-orangeLight transition-colors font-bold"
            >
              Security handbook
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                class="h-4 w-4"
              >
                <line x1="5" y1="12" x2="19" y2="12" />
                <polyline points="12 5 19 12 12 19" />
              </svg>
            </a>
          </div>
        </article>
      </div>
    </div>
  `,
})
export default class WindowsPage implements OnInit {
  constructor(
    private title: Title,
    private meta: Meta
  ) {}

  ngOnInit(): void {
    this.title.setTitle('Windows Developer Guide — Crush');
    this.meta.updateTag({
      name: 'description',
      content:
        'Crush on Windows — no WSL2 required. Deep dive into Win32 Job Objects kernel sandboxing, local step-by-step setup guides, and microVM compilation routines.',
    });
  }
}
