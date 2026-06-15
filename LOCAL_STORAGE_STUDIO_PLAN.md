# Crush → Local Object-Storage Studio (S3/R2) — Implementation Plan (Plan #3)

**Thesis (validated):** crush already runs **MinIO** natively (S3-compatible, `crates/crush-services/src/minio.rs`, `inspect_minio`) and detects S3/Cloudflare-R2 as external services. Turn that into a **local S3 studio** — browse/manage buckets and objects — that works against *both* the crush-managed MinIO **and any S3-compatible endpoint** (AWS S3, Cloudflare R2, Backblaze B2, Wasabi, Supabase Storage's S3 API) via credentials. Think "TablePlus for object storage," local and free.

**Same ground rules as `LOCAL_PAAS_PLAN.md`** (read its "Ground rules" + "Architecture cheat-sheet" first). Solo commits on `main`, no co-author trailers, disk-tight box, verify `cargo build` + `svelte-check` (baseline = the 2 env errors), **no stubs — wire every command to visible UI**, Windows-first, don't push `ci`/release (validator does). One phase per commit.

## Scope decision (validated — what fits crush vs not)

| Supabase Storage capability | Crush scope | Why |
|---|---|---|
| **Files buckets — browse/upload/download/delete objects, manage buckets** | **IN — the core** | Pure local dev tool over an S3 API crush already runs (MinIO) + any S3 endpoint. |
| **S3-compatible multi-endpoint** (MinIO, AWS S3, R2, B2, Wasabi) | IN | One credentials form → manage any of them. Big practical value. |
| **Direct URL / presigned URLs** | IN | Generate share/upload URLs locally; standard S3. |
| **Image preview + basic local transforms** (resize/convert on download) | IN (light, late) | Useful dev convenience done locally. NOT a CDN transform service. |
| **Fine-grained access (bucket policies / public toggle)** | IN (partial) | S3 bucket policy JSON + public/private toggle. (Supabase's RLS is DB-side — belongs to Plan #2, not here.) |
| **Resumable uploads (TUS)** | OUT→optional | Use S3 **multipart** for large-file reliability instead; TUS protocol itself is not worth it. |
| **Global CDN (285 cities)** | OUT | A CDN business, not a local tool. |
| **Analytics buckets (Apache Iceberg, SQL foreign tables)** | OUT | Niche, heavy, data-lake territory; revisit only on real demand. |
| **Vector buckets** | OUT (here) | The vector use-case is **pgvector** in Plan #2 (DB studio), not object storage. |

## Foundations (build first)

**Backend S3 layer.** Add an `s3` capability (module in `crates/crush-services` or a small `crush-storage` crate). Two viable drivers — **decide with the validator before adding a dep** (Windows cross-compile + build size matter):
- `rust-s3` (`s3` crate): lightweight, endpoint-agnostic, works with MinIO/R2/AWS. Preferred.
- `aws-sdk-s3`: heavier but battle-tested.
- Fallback with zero new deps: shell out to the **MinIO client `mc`** or **`aws` CLI** (provision `mc` on demand like other tools). Acceptable for v1 if a Rust SDK is deemed too heavy.

**Connection model.** A saved list of **storage connections**: `{ name, endpoint, region, access_key, secret_key, path_style }`. Defaults:
- crush-managed MinIO auto-added (endpoint `http://localhost:9000`, creds from the service config — reuse `connection_string_for("minio", …)` / service state).
- "Add connection" form for S3/R2/etc. Creds stored with the existing config mechanism; **never logged** (redacted Debug).

**Frontend.** A `/storage` page (add to `Sidebar.svelte`, with a `database`-style or new `archive` icon — verify the Icon exists or add a lucide path). Left: connection switcher + bucket list. Right: object browser.

## Phase S1 — Buckets + object browser (daily driver)
- **Buckets:** list, **create**, **delete** (confirm; block if non-empty unless "force"), see object count/size if cheap.
- **Objects:** browse by prefix (folder tree from `/`-delimited keys), list with size + last-modified + content-type, **download**, **delete** (single + multi-select), **upload** (file picker + drag-drop; multipart for large files).
- **Preview:** images/text inline; everything else → download.
- **Accept:** open crush MinIO, create a bucket, upload a file, see it, preview an image, download it, delete it.

## Phase S2 — URLs + sharing
- **Copy object URL** (direct, for public buckets) and **presigned GET** (temporary, configurable TTL).
- **Presigned PUT** for handing an upload URL to someone/something.
- **Accept:** generate a presigned GET that opens the object in a browser; expires after TTL.

## Phase S3 — Access control
- Per-bucket **public/private** toggle (bucket policy for `s3:GetObject`).
- View/edit raw **bucket policy** JSON (advanced).
- **Accept:** make a bucket public, confirm an object opens via direct URL; flip back to private.

## Phase S4 — Niceties (late / optional)
- **Local image transforms** on download (resize/convert) — done locally, not a hosted transform endpoint.
- **Sync helpers:** mirror a local folder ↔ a bucket (great for deploys/static sites). Reuse `mc mirror` / SDK.
- **Object metadata** view/edit (content-type, cache-control, custom metadata).
- **Accept:** resize-on-download produces a smaller image; folder→bucket sync uploads changed files only.

## Per-phase definition of done
Same as Plans #1/#2: Rust builds, svelte-check only-baseline, unit tests for pure logic (key→prefix tree building, presign URL construction if done in-house, multipart chunking math), **feature visible+usable in GUI**, commit per phase, hand to validator. No version bumps / releases.

## Reliability musts (real data + credentials)
- Credentials never logged; redacted Debug; stored via the existing secret-handling path.
- Uploads/downloads stream (don't buffer whole files in memory); large files use multipart with retry per part.
- Deletes always confirm; multi-delete shows the count.
- Endpoint/auth failures surface a clear message (wrong key, bucket missing, CORS) — never a silent hang; every call has a timeout.
- `path_style` vs virtual-host addressing configurable (MinIO needs path-style; AWS/R2 vary) — auto-default path-style for `localhost`/MinIO.
- Treat the bucket as a remote pooled service (matches the DB-studio guidance).
