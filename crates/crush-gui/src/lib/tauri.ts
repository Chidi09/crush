import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export interface PortInfo {
  host_port: number;
  container_port: number;
}

export interface ContainerSummary {
  id: string;
  name: string;
  image: string;
  status: string;
  pid: number | null;
  created_at: number;
  cpu_percent: number | null;
  memory_bytes: number | null;
  uptime_secs: number | null;
  ports: PortInfo[];
}

export interface NativeServiceSummary {
  project: string;
  name: string;
  kind: string;
  port: number;
  pid: number;
  connection_string: string | null;
  data_dir: string;
  started_at: number;
  console_url: string | null;
}

export interface ImageSummary {
  id: string;
  tag: string;
  digest: string;
  size_bytes: number;
  layer_count: number;
  os: string;
  arch: string;
  stack?: string | null;
}

export interface ImageDetail {
  id: string;
  tag: string;
  digest: string;
  size_bytes: number;
  os: string;
  arch: string;
  entrypoint: string[];
  cmd: string[];
  env: string[];
  layers: string[];
  config_digest: string | null;
}

export interface BuildSummary {
  timestamp_ms: number;
  project_name: string;
  language: string;
  framework: string;
  duration_ms: number;
  was_cached: boolean;
  size_bytes: number;
  digest: string;
  success: boolean;
}

export interface DiagnosisResult {
  summary: string;
  details: string | null;
  fix: string | null;
}

export interface ExternalService {
  name: string;
  kind: string;
  source_var: string;
}

export interface ProjectInfo {
  name: string;
  runtime: string;
  version: string;
  framework: string | null;
  port: number;
  confidence: number;
  is_monorepo: boolean;
  env_required: string[];
  service_count: number;
  stack_kind: string | null;
  external_services?: ExternalService[];
}

export interface DiskSegment {
  label: string;
  bytes: number;
}

export interface SystemInfo {
  version: string;
  os: string;
  arch: string;
  data_dir: string;
  disk_used_bytes: number;
  disk_breakdown: DiskSegment[];
}

export interface LogLine {
  ts: string;
  stream: string;
  text: string;
}

// RunEvent types matching the Rust RunEvent enum
export interface RunEventDetected {
  kind: 'detected';
  language: string;
  framework: string;
  confidence: number;
  is_monorepo: boolean;
  port: number;
  dep_count: number;
}

export interface RunEventBuildOutput {
  kind: 'build-output';
  line: string;
  stream: 'stdout' | 'stderr';
  service_name: string | null;
}

export interface RunEventAppOutput {
  kind: 'app-output';
  line: string;
  stream: 'stdout' | 'stderr';
  service_name: string | null;
}

export interface RunEventDepStarted {
  kind: 'dep-started';
  name: string;
  image: string;
  native: boolean;
}

export interface RunEventDepFailed {
  kind: 'dep-failed';
  name: string;
  error: string;
}

export interface RunEventImageFresh {
  kind: 'image-fresh';
  digest: string;
}

export interface RunEventImagePacked {
  kind: 'image-packed';
  digest: string;
  size_bytes: number;
  duration_ms: number;
}

export interface RunEventBuildStarted {
  kind: 'build-started';
  command: string;
  service_name: string | null;
}

export interface RunEventBuildFinished {
  kind: 'build-finished';
  duration_ms: number;
  success: boolean;
  service_name: string | null;
}

export interface RunEventSpawning {
  kind: 'spawning';
  command: string;
  port: number;
  service_name: string | null;
}

export interface RunEventWarning {
  kind: 'warning';
  message: string;
}

export interface RunEventWarmRun {
  kind: 'warm-run';
}

export interface RunEventDepsFresh {
  kind: 'deps-fresh';
}

export interface RunEventAborted {
  kind: 'aborted';
}

export interface RunEventExited {
  kind: 'exited';
  code: number;
}

export interface RunEventPortBound {
  kind: 'port-bound';
  port: number;
  startup_ms: number;
  total_ms: number;
  urls: [string, string][];
  service_name: string | null;
}

export type RunEvent =
  | RunEventDetected
  | RunEventDepStarted
  | RunEventDepFailed
  | RunEventImageFresh
  | RunEventImagePacked
  | RunEventBuildStarted
  | RunEventBuildOutput
  | RunEventBuildFinished
  | RunEventSpawning
  | RunEventAppOutput
  | RunEventPortBound
  | RunEventExited
  | RunEventWarning
  | RunEventWarmRun
  | RunEventDepsFresh
  | RunEventAborted;
// Tauri commands
export function listContainers(): Promise<ContainerSummary[]> {
  return invoke('list_containers');
}

export function stopContainer(id: string): Promise<void> {
  return invoke('stop_container', { id });
}

export function listNativeServices(): Promise<NativeServiceSummary[]> {
  return invoke('list_native_services');
}

export function stopNativeService(name: string, project: string): Promise<void> {
  return invoke('stop_native_service', { name, project });
}

export function getConnectionString(name: string, project: string): Promise<string | null> {
  return invoke('get_connection_string', { name, project });
}

export function listImages(): Promise<ImageSummary[]> {
  return invoke('list_images');
}

export function inspectImage(id: string): Promise<ImageDetail> {
  return invoke('inspect_image', { id });
}

export function pullImage(reference: string): Promise<string> {
  return invoke('pull_image', { reference });
}

export function removeImage(id: string): Promise<void> {
  return invoke('remove_image', { id });
}

export function runProject(projectPath: string, devMode: boolean): Promise<string> {
  return invoke('run_project', { projectPath, devMode });
}

// ── Android device / emulator mirroring (mobile run view) ──
export interface AdbDevice { serial: string; state: string; is_emulator: boolean; }

export function adbDevices(): Promise<AdbDevice[]> {
  return invoke('adb_devices');
}
/** PNG data: URL of the device screen. */
export function deviceScreencap(serial = ''): Promise<string> {
  return invoke('device_screencap', { serial });
}
export function deviceTap(serial: string, x: number, y: number): Promise<void> {
  return invoke('device_tap', { serial, x: Math.round(x), y: Math.round(y) });
}
export function deviceSwipe(serial: string, x1: number, y1: number, x2: number, y2: number, ms = 200): Promise<void> {
  return invoke('device_swipe', { serial, x1: Math.round(x1), y1: Math.round(y1), x2: Math.round(x2), y2: Math.round(y2), ms });
}
/** Android keyevent: 4=BACK, 3=HOME, 187=APP_SWITCH. */
export function deviceKey(serial: string, keycode: number): Promise<void> {
  return invoke('device_key', { serial, keycode });
}

export function abortRun(runId: string): Promise<void> {
  return invoke('abort_run', { runId });
}

export function subscribeLogs(containerId: string): Promise<void> {
  return invoke('subscribe_logs', { containerId });
}

export function unsubscribeLogs(containerId: string): Promise<void> {
  return invoke('unsubscribe_logs', { containerId });
}

export function readServiceLog(project: string, name: string, maxLines?: number): Promise<string> {
  return invoke('read_service_log', { project, name, maxLines });
}

// Service inspection (tables / connections / keys)
export interface PgTable { schema: string; name: string; rows: number }
export interface PgConn { pid: number; user: string; db: string; state: string; query: string }
export interface PgInspect { version: string; current_db: string; databases: string[]; tables: PgTable[]; connections: PgConn[] }
export interface RedisKey { key: string; kind: string; ttl: number }
export interface RedisInspect { total: number; keys: RedisKey[] }

export function inspectPostgres(port: number, user?: string, password?: string, database?: string): Promise<PgInspect> {
  return invoke('inspect_postgres', { port, user, password, database });
}
export function inspectRedis(port: number, password?: string): Promise<RedisInspect> {
  return invoke('inspect_redis', { port, password });
}
export interface MongoColl { name: string; count: number }
export interface MongoDb { name: string; collections: MongoColl[] }
export interface MongoInspect { databases: MongoDb[] }
export interface S3Bucket { name: string; objects: number; size: number }
export interface MinioInspect { buckets: S3Bucket[] }

export function inspectMongo(port: number): Promise<MongoInspect> {
  return invoke('inspect_mongo', { port });
}
export function inspectMinio(port: number, user?: string, password?: string): Promise<MinioInspect> {
  return invoke('inspect_minio', { port, user, password });
}

export function listBuildHistory(limit?: number): Promise<BuildSummary[]> {
  return invoke('list_build_history', { limit: limit ?? 50 });
}

export function diagnoseLogs(lines: string[]): Promise<DiagnosisResult> {
  return invoke('diagnose_logs', { lines });
}

export function pickProjectDirectory(): Promise<string | null> {
  return invoke('pick_project_directory');
}

export function openUrl(url: string): Promise<void> {
  return invoke('open_url', { url });
}

export function revealInExplorer(path: string): Promise<void> {
  return invoke('reveal_in_explorer', { path });
}

export function detectProject(path: string): Promise<ProjectInfo> {
  return invoke('detect_project', { path });
}

export function systemInfo(): Promise<SystemInfo> {
  return invoke('system_info');
}

export interface ResourceUsage {
  cpu_percent: number;
  mem_used_bytes: number;
  mem_total_bytes: number;
}
export function systemResources(): Promise<ResourceUsage> {
  return invoke('system_resources');
}

// Deployments (persisted run history, Vercel-style)
export interface DeploymentRecord {
  id: string;
  project: string;
  project_path: string;
  created_ms: number;
  ended_ms: number | null;
  duration_ms: number;
  status: 'running' | 'ready' | 'failed';
  port: number | null;
  runtime: string | null;
  framework: string | null;
  build_log: string;
  runtime_log: string;
  has_screenshot: boolean;
  branch?: string;
  commit_short?: string;
  commit_message?: string;
}
export interface DeploymentDetail extends DeploymentRecord {
  screenshot: string | null;
}

export function saveDeployment(record: DeploymentRecord): Promise<void> {
  return invoke('save_deployment', { record });
}
export function listDeployments(project: string): Promise<DeploymentRecord[]> {
  return invoke('list_deployments', { project });
}
export function getDeployment(project: string, id: string): Promise<DeploymentDetail> {
  return invoke('get_deployment', { project, id });
}
export function deleteDeployment(project: string, id: string): Promise<void> {
  return invoke('delete_deployment', { project, id });
}
export function listAllDeployments(): Promise<DeploymentRecord[]> {
  return invoke('list_all_deployments');
}

export interface EjectResult {
  dockerfile: string;
  compose: string;
}
export function ejectProject(path: string, force: boolean): Promise<EjectResult> {
  return invoke('eject_project', { path, force });
}
export function capturePreview(project: string, id: string, x: number, y: number, w: number, h: number): Promise<string | null> {
  return invoke('capture_preview', { project, id, x, y, w, h });
}

// Deploy (eject-to-provider + wrap official CLIs)
export function writeProjectFile(path: string, filename: string, content: string): Promise<string> {
  return invoke('write_project_file', { path, filename, content });
}
export function cliAvailable(program: string, probe: string): Promise<boolean> {
  return invoke('cli_available', { program, probe });
}
export function runDeploy(path: string, program: string, args: string[], env: Record<string, string>): Promise<void> {
  return invoke('run_deploy', { path, program, args, env });
}
export function runCapture(path: string, program: string, args: string[], env: Record<string, string>): Promise<string> {
  return invoke('run_capture', { path, program, args, env });
}
export function openTerminal(path: string, command: string): Promise<void> {
  return invoke('open_terminal', { path, command });
}
export function onDeployLine(cb: (e: { stream: string; line: string }) => void): Promise<UnlistenFn> {
  return listen<{ stream: string; line: string }>('deploy-line', (e) => cb(e.payload));
}
export function onDeployExit(cb: (e: { code: number }) => void): Promise<UnlistenFn> {
  return listen<{ code: number }>('deploy-exit', (e) => cb(e.payload));
}

// Git
export interface GithubRepo {
  owner: string;
  repo: string;
}
export interface GitCommit {
  short: string;
  message: string;
  author: string;
  committed_rel: string;
  committed_ms: number;
}
export interface GitInfo {
  is_repo: boolean;
  branch: string | null;
  remote_url: string | null;
  parsed_github: GithubRepo | null;
  head: GitCommit | null;
  dirty_count: number;
  ahead: number | null;
  behind: number | null;
  upstream: string | null;
}
export interface BranchInfo {
  name: string;
  is_current: boolean;
  is_remote: boolean;
  short: string | null;
  message: string | null;
  author: string | null;
  committed_rel: string | null;
  committed_ms: number | null;
}

export function gitInfo(path: string): Promise<GitInfo> {
  return invoke('git_info', { path });
}
export function gitBranches(path: string, fetch: boolean): Promise<BranchInfo[]> {
  return invoke('git_branches', { path, fetch });
}
export function previewBranch(path: string, branch: string): Promise<string> {
  return invoke('preview_branch', { path, branch });
}
export function removeWorktree(path: string, branch: string): Promise<void> {
  return invoke('remove_worktree', { path, branch });
}


// Event listeners
export function onRunEvent(runId: string, cb: (event: RunEvent) => void): Promise<UnlistenFn> {
  return listen<RunEvent>(`run-event::${runId}`, (e) => cb(e.payload));
}

export function onLogLine(containerId: string, cb: (line: LogLine) => void): Promise<UnlistenFn> {
  return listen<LogLine>(`log-line::${containerId}`, (e) => cb(e.payload));
}

export function onLogReplay(containerId: string, cb: (lines: LogLine[]) => void): Promise<UnlistenFn> {
  return listen<LogLine[]>(`log-replay::${containerId}`, (e) => cb(e.payload));
}

export function onContainerStateChanged(cb: () => void): Promise<UnlistenFn> {
  return listen('container-state-changed', () => cb());
}

export function onServiceStateChanged(cb: () => void): Promise<UnlistenFn> {
  return listen('service-state-changed', () => cb());
}

// Config / Settings
export interface AppConfig {
  ai_provider: string;
  ai_api_key: string;
  ai_model: string;
  auto_diagnose: boolean;
  default_provider: string;
  default_region: string;
  postgres_port: number;
  redis_port: number;
  mongo_port: number;
  minio_port: number;
  services_data_dir: string;
  auto_stop_services: boolean;
  reduce_motion: boolean;
  accent_color: string;
  check_for_updates: boolean;
}

export function getConfig(): Promise<AppConfig> {
  return invoke('get_config');
}

export function setConfig(config: AppConfig): Promise<void> {
  return invoke('set_config', { config });
}

