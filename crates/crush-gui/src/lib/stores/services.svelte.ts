import { writable } from 'svelte/store';
import type { NativeServiceSummary } from '$lib/tauri';
import * as api from '$lib/tauri';

export const services = writable<NativeServiceSummary[]>([]);
export const loading = writable(true);

export async function refreshServices() {
  try {
    const list = await api.listNativeServices();
    services.set(list);
    loading.set(false);
  } catch (e) {
    console.error('Failed to list services', e);
    loading.set(false);
  }
}
