<script lang="ts">
  // Live mirror of an Android emulator/device for the mobile run view.
  // The emulator is a separate OS window we can't embed, so we poll its screen
  // (adb exec-out screencap -p) and forward taps/swipes (adb shell input) — a
  // "good enough" interactive view of the running Flutter / React Native app.
  import { onDestroy } from 'svelte';
  import * as api from '$lib/tauri';

  let devices = $state<api.AdbDevice[]>([]);
  let serial = $state('');
  let frame = $state<string>('');     // PNG data URL
  let err = $state<string>('');
  let live = $state(false);
  let fps = $state(2);                 // polls/sec
  let imgEl = $state<HTMLImageElement | null>(null);

  // pointer→device-pixel mapping uses the image's natural (device) size.
  let dragStart: { x: number; y: number; t: number } | null = null;

  let timer: ReturnType<typeof setInterval> | null = null;

  async function refreshDevices() {
    try {
      devices = await api.adbDevices();
      const online = devices.filter((d) => d.state === 'device');
      if (!serial && online.length) serial = online[0].serial;
      err = devices.length ? '' : 'no device/emulator — boot one, then Refresh';
    } catch (e) {
      err = String(e);
    }
  }

  async function tick() {
    try {
      frame = await api.deviceScreencap(serial);
      err = '';
    } catch (e) {
      err = String(e);
    }
  }

  function start() {
    if (timer) clearInterval(timer);
    live = true;
    tick();
    timer = setInterval(tick, Math.max(200, Math.round(1000 / fps)));
  }
  function stop() {
    live = false;
    if (timer) { clearInterval(timer); timer = null; }
  }

  // Map a pointer event on the <img> to device pixels via naturalWidth/Height.
  function toDevice(ev: PointerEvent): { x: number; y: number } | null {
    const el = imgEl;
    if (!el || !el.naturalWidth) return null;
    const r = el.getBoundingClientRect();
    const x = ((ev.clientX - r.left) / r.width) * el.naturalWidth;
    const y = ((ev.clientY - r.top) / r.height) * el.naturalHeight;
    return { x, y };
  }

  function onDown(ev: PointerEvent) {
    const p = toDevice(ev);
    if (p) dragStart = { ...p, t: Date.now() };
  }
  async function onUp(ev: PointerEvent) {
    const p = toDevice(ev);
    if (!p || !dragStart) { dragStart = null; return; }
    const dx = p.x - dragStart.x, dy = p.y - dragStart.y;
    const dist = Math.hypot(dx, dy);
    try {
      if (dist < 12) {
        await api.deviceTap(serial, p.x, p.y);
      } else {
        await api.deviceSwipe(serial, dragStart.x, dragStart.y, p.x, p.y, Math.min(600, Date.now() - dragStart.t));
      }
      if (!live) tick(); // refresh once if not streaming
    } catch (e) {
      err = String(e);
    }
    dragStart = null;
  }

  refreshDevices();
  onDestroy(stop);
</script>

<div class="device-view">
  <div class="bar">
    <select bind:value={serial} title="device">
      {#each devices as d}
        <option value={d.serial}>{d.is_emulator ? '📱' : '🔌'} {d.serial} ({d.state})</option>
      {/each}
      {#if !devices.length}<option value="">no devices</option>{/if}
    </select>
    <button onclick={refreshDevices} title="re-scan devices">⟳</button>
    {#if live}
      <button onclick={stop}>⏸ stop</button>
    {:else}
      <button onclick={start} disabled={!serial}>▶ mirror</button>
    {/if}
    <label class="fps">{fps} fps<input type="range" min="1" max="10" bind:value={fps} onchange={() => live && start()} /></label>
  </div>

  <div class="screen">
    {#if frame}
      <img
        bind:this={imgEl}
        src={frame}
        alt="device screen"
        onpointerdown={onDown}
        onpointerup={onUp}
        draggable="false"
      />
    {:else}
      <div class="placeholder">{err || 'press ▶ mirror to view the running app'}</div>
    {/if}
  </div>

  <div class="keys">
    <button onclick={() => api.deviceKey(serial, 4)} disabled={!serial} title="Back">◀</button>
    <button onclick={() => api.deviceKey(serial, 3)} disabled={!serial} title="Home">●</button>
    <button onclick={() => api.deviceKey(serial, 187)} disabled={!serial} title="Recents">■</button>
    {#if err && frame}<span class="err">{err}</span>{/if}
  </div>
</div>

<style>
  .device-view { display: flex; flex-direction: column; gap: 8px; align-items: center; }
  .bar { display: flex; gap: 6px; align-items: center; flex-wrap: wrap; width: 100%; }
  .bar select, .bar button { background: var(--color-crush-surface); color: var(--color-crush-text);
    border: 1px solid var(--color-crush-border); border-radius: 6px; padding: 4px 8px; cursor: pointer; }
  .bar button:disabled { opacity: 0.5; cursor: not-allowed; }
  .fps { display: flex; align-items: center; gap: 4px; font-size: 12px; color: var(--color-crush-text-dim); margin-left: auto; }
  .screen { border: 1px solid var(--color-crush-border); border-radius: 18px; overflow: hidden;
    background: #000; max-height: 70vh; display: flex; }
  .screen img { display: block; max-height: 70vh; width: auto; touch-action: none; user-select: none; }
  .placeholder { color: var(--color-crush-text-dim); padding: 48px 24px; font-size: 13px; text-align: center; }
  .keys { display: flex; gap: 10px; align-items: center; }
  .keys button { width: 40px; height: 28px; border-radius: 6px; background: var(--color-crush-surface);
    color: var(--color-crush-text); border: 1px solid var(--color-crush-border); cursor: pointer; }
  .err { color: #f87171; font-size: 12px; }
</style>
