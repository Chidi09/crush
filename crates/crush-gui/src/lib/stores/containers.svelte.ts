import { writable } from 'svelte/store';
import type { ContainerSummary } from '$lib/tauri';
import * as api from '$lib/tauri';

export const containers = writable<ContainerSummary[]>([]);
export const loading = writable(true);

let intervalId: ReturnType<typeof setInterval> | null = null;

export function startPolling() {
  refresh();
  intervalId = setInterval(refresh, 2000);
}

export function stopPolling() {
  if (intervalId) {
    clearInterval(intervalId);
    intervalId = null;
  }
}

async function refresh() {
  try {
    const list = await api.listContainers();
    containers.set(list);
    loading.set(false);
  } catch (e) {
    console.error('Failed to list containers', e);
    loading.set(false);
  }
}
