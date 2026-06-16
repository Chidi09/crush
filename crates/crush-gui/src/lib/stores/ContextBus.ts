import { writable } from 'svelte/store';

export const contextBus = writable<{
  activeTable?: string;
  activeServer?: string;
  activeBucket?: string;
}>({});
