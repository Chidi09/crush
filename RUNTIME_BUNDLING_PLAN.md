# Auto-install runtimes (next big feature)

Same pattern as `BinaryCache` already does for portable postgres/garnet.
For every runtime crush needs to spawn, check PATH first; if missing,
download a portable build into `%LOCALAPPDATA%\Crush\cache\runtimes\<name>\<version>\`
and prepend that to the child process's PATH.

## Runtimes to cover (priority order)
| Runtime | Source (Windows x64)                                                                |
|---------|-------------------------------------------------------------------------------------|
| Node    | https://nodejs.org/dist/v20.x.x/node-v20.x.x-win-x64.zip                             |
| pnpm    | npm-install once node is present, OR direct https://github.com/pnpm/pnpm/releases    |
| Python  | https://www.python.org/ftp/python/3.12.x/python-3.12.x-embed-amd64.zip               |
| uv      | https://github.com/astral-sh/uv/releases (already a single .exe, easy)               |
| JDK     | https://api.adoptium.net/v3/binary/latest/21/ga/windows/x64/jre/hotspot/normal/eclipse |
| Maven   | https://archive.apache.org/dist/maven/maven-3/3.9.x/binaries/apache-maven-3.9.x-bin.zip |
| Go      | https://go.dev/dl/go1.x.x.windows-amd64.zip                                          |
| Rust    | rustup (https://win.rustup.rs/x86_64) — heavier, defer                               |
| Bun     | https://github.com/oven-sh/bun/releases                                              |
| Deno    | https://github.com/denoland/deno/releases                                            |

## Design
1. **New crate or module**: `crush-runtimes` (sibling of `crush-services`).
   Reuse existing `BinaryCache` from `crush-services` — it already handles
   download + SHA256 + zip/tar/exe extraction + sentinel.
2. **`RuntimeSpec`** struct: name, version, download url, SHA256, expected
   binary subpath (e.g. `node-v20.../node.exe`).
3. **`ensure_runtime(name) -> PathBuf`**: returns the directory to prepend
   to PATH. Idempotent — sentinel file means subsequent calls are zero-cost.
4. **Hook in main.rs `spawn_shell`**: before spawning, walk the cmdline's
   first token (`mvn`, `pnpm`, `uvicorn` after resolving to `.venv\Scripts`).
   If not on PATH and a known runtime owns it, call `ensure_runtime(...)`
   and merge into env.

## Detection
- Check PATH via `which::which(name).is_ok()`.
- If absent AND the project needs it (stack.language match), trigger fetch.

## Versioning
Use the runtime version detected by `VersionResolver` when available
(e.g. `engines.node`, `requires-python`, `<java.version>` in pom). Fall
back to a sensible LTS default if unparseable.

## UX
On first install: `   ↳ fetching node 20.18.0 (32 MB)...  ok` style line,
matching the postgres bootstrap line.

## Open questions
- Disk usage cap? A user with many projects could collect 5+ runtimes.
  Probably fine, but worth a `crush prune` command later.
- Symlink vs PATH-prefix? PATH-prefix is simpler and avoids admin needs.
