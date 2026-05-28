import { writable } from 'svelte/store';
import type { ImageSummary } from '$lib/tauri';
import * as api from '$lib/tauri';

export const images = writable<ImageSummary[]>([]);
export const loading = writable(true);

export async function refreshImages() {
  try {
    const list = await api.listImages();
    images.set(list);
    loading.set(false);
  } catch (e) {
    console.error('Failed to list images', e);
    loading.set(false);
  }
}
