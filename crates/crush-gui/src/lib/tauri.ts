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
}

export interface ImageSummary {
  id: string;
  tag: string;
  digest: string;
  size_bytes: number;
  layer_count: number;
  created_at: string;
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
}

export interface SystemInfo {
  version: string;
  os: string;
  arch: string;
  data_dir: string;
  disk_used_bytes: number;
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

export type RunEvent = RunEventDetected | RunEventBuildOutput | RunEventAppOutput | RunEventExited | RunEventPortBound;

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

export function pullImage(reference: string): Promise<string> {
  return invoke('pull_image', { reference });
}

export function removeImage(id: string): Promise<void> {
  return invoke('remove_image', { id });
}

export function runProject(projectPath: string): Promise<string> {
  return invoke('run_project', { projectPath });
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

// Event listeners
export function onRunEvent(runId: string, cb: (event: RunEvent) => void): Promise<UnlistenFn> {
  return listen<RunEvent>(`run-event::${runId}`, (e) => cb(e.payload));
}

export function onLogLine(containerId: string, cb: (line: LogLine) => void): Promise<UnlistenFn> {
  return listen<LogLine>(`log-line::${containerId}`, (e) => cb(e.payload));
}

export function onContainerStateChanged(cb: () => void): Promise<UnlistenFn> {
  return listen('container-state-changed', () => cb());
}

export function onServiceStateChanged(cb: () => void): Promise<UnlistenFn> {
  return listen('service-state-changed', () => cb());
}
