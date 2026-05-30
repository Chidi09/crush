<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import Icon from './Icon.svelte';
  import TechIcon from './TechIcon.svelte';
  import * as api from '$lib/tauri';
  import { toast } from '$lib/stores/toast.svelte.ts';
  import { findTarget, parseDeployUrl, recommendFor, classifyStack, type DeployStack, type DeployTarget } from '$lib/deploy/targets';

  let { path, stack, onClose }: { path: string; stack: DeployStack; onClose: () => void } = $props();

  let targetId = $state<string | null>(null);
  let target = $derived(targetId ? findTarget(targetId) : undefined);

  // Recommend targets based on the detected stack (frontend → Vercel/Netlify/…,
  // backend → Railway/Fly/…). Fullstack frameworks see everything.
  let kind = $derived(classifyStack(stack));
  let rec = $derived(recommendFor(kind));
  const groupLabel = (g: string) => g === 'registry' ? 'registry push' : g === 'frontend' ? 'framework build' : g === 'vps' ? 'VM + Caddy' : 'builds Dockerfile';
  let showOthers = $state(false);
  let fields = $state<Record<string, string>>({});
  let token = $state('');

  let generatedConfig = $state<string | null>(null);
  let writtenPath = $state<string | null>(null);
  let generating = $state(false);
  let genErr = $state<string | null>(null);

  let deploying = $state(false);
  let logLines = $state<string[]>([]);
  let logEl: HTMLDivElement | undefined = $state();
  let deployUrl = $state<string | null>(null);
  let exitCode = $state<number | null>(null);
  let cliMissing = $state(false);
  let unlistenLine: (() => void) | null = null;
  let unlistenExit: (() => void) | null = null;

  let commandStr = $derived.by(() => {
    if (!target?.command) return null;
    const c = target.command(stack, fields);
    return c ? `${c.program} ${c.args.join(' ')}` : null;
  });

  function selectTarget(id: string) {
    targetId = id;
    const t = findTarget(id)!;
    const f: Record<string, string> = {};
    for (const fld of t.fields) f[fld.key] = fld.def ? fld.def(stack) : '';
    fields = f;
    token = '';
    generatedConfig = null; writtenPath = null; genErr = null;
    logLines = []; deployUrl = null; exitCode = null; cliMissing = false;
  }

  async function generate() {
    if (!target) return;
    generating = true; genErr = null;
    try {
      if (target.needsDockerfile) {
        // Ensure a Dockerfile exists (idempotent — ignore "already exists").
        await api.ejectProject(path, false).catch((e) => {
          if (!/already exists/i.test(String((e as any)?.message ?? e))) throw e;
        });
      }
      if (target.configFile && target.config) {
        const content = target.config(stack, fields);
        generatedConfig = content;
        writtenPath = await api.writeProjectFile(path, target.configFile, content);
        toast(`Wrote ${target.configFile}`, 'success');
      } else {
        generatedConfig = '(no config file — uses CLI flags only)';
      }
    } catch (e) {
      genErr = String((e as any)?.message ?? e);
      toast('Generate failed', 'error');
    } finally {
      generating = false;
    }
  }

  async function deploy() {
    if (!target || !target.command) return;
    const cmd = target.command(stack, fields);
    if (!cmd) return;
    cliMissing = false;
    const ok = await api.cliAvailable(target.cli, target.cliProbe).catch(() => false);
    if (!ok) { cliMissing = true; return; }

    deploying = true; logLines = []; deployUrl = null; exitCode = null;
    const env = target.env ? target.env(token, fields) : {};

    unlistenLine = await api.onDeployLine(async (e) => {
      logLines = [...logLines.slice(-800), e.line];
      await tick();
      logEl?.scrollTo({ top: logEl.scrollHeight });
    });
    unlistenExit = await api.onDeployExit((e) => {
      exitCode = e.code;
      deploying = false;
      deployUrl = parseDeployUrl(logLines.join('\n'));
      toast(e.code === 0 ? (deployUrl ? 'Deployed — live URL ready' : 'Deploy finished') : `Deploy exited (code ${e.code})`, e.code === 0 ? 'success' : 'error');
      unlistenLine?.(); unlistenExit?.(); unlistenLine = null; unlistenExit = null;
    });

    try {
      await api.runDeploy(path, cmd.program, cmd.args, env);
    } catch (e) {
      logLines = [...logLines, `error: ${String((e as any)?.message ?? e)}`];
      deploying = false;
    }
  }

  function copyCmd() { if (commandStr) { navigator.clipboard.writeText(commandStr).catch(() => {}); toast('Command copied', 'success'); } }
  function visit() { if (deployUrl) api.openUrl(deployUrl).catch(() => {}); }
  // Pop a real console for browser-based login / interactive prompts the GUI
  // can't drive headlessly.
  function authenticate() { if (target?.auth) { api.openTerminal(path, target.auth).catch(() => {}); toast(`Opening terminal: ${target.auth}`, 'info'); } }
  function openInTerminal() { if (commandStr) { api.openTerminal(path, commandStr).catch(() => {}); toast('Opening terminal…', 'info'); } }

  onDestroy(() => { unlistenLine?.(); unlistenExit?.(); });
</script>

<button class="overlay" onclick={onClose} aria-label="Close"></button>
<div class="modal animate-slide-up">
  <div class="m-head">
    <div class="m-title"><Icon name="rocket" size={15} /><h2>Deploy {stack.name}</h2></div>
    <button class="close-btn" onclick={onClose} aria-label="Close">×</button>
  </div>

  <div class="m-body">
    <!-- Target picker (recommended for the detected stack first) -->
    {#snippet targetBtn(t: DeployTarget)}
      <button class="target" class:active={targetId === t.id} onclick={() => selectTarget(t.id)}>
        <span class="t-top">
          {#if t.icon}
            <TechIcon name={t.icon} size={18} />
          {:else}
            <span class="mono" style="background:{t.accent ?? '#444'}">{t.mono}</span>
          {/if}
          <span class="t-label">{t.label}</span>
        </span>
        <span class="t-group {t.group}">{groupLabel(t.group)}</span>
      </button>
    {/snippet}

    <div class="rec-head">
      Recommended for <span class="kind">{kind}</span>
      <span class="kind-sub">{stack.framework ?? stack.runtime ?? 'app'}</span>
    </div>
    <div class="targets">
      {#each rec.recommended as t}{@render targetBtn(t)}{/each}
    </div>

    {#if rec.others.length}
      <button class="others-toggle" onclick={() => showOthers = !showOthers}>
        {showOthers ? '▾' : '▸'} {rec.others.length} other target{rec.others.length === 1 ? '' : 's'}
      </button>
      {#if showOthers}
        <div class="targets dim-targets">
          {#each rec.others as t}{@render targetBtn(t)}{/each}
        </div>
      {/if}
    {/if}

    {#if target}
      <div class="config-panel">
        <!-- Token + fields -->
        <div class="fields">
          <label class="fld">
            <span class="fk">{target.tokenLabel} <span class="env">${target.tokenEnv}</span></span>
            <input type="password" bind:value={token} placeholder="paste token (kept in memory, not saved)" />
          </label>
          {#each target.fields as fld}
            <label class="fld">
              <span class="fk">{fld.label}</span>
              <input type="text" bind:value={fields[fld.key]} placeholder={fld.placeholder ?? ''} />
            </label>
          {/each}
        </div>

        {#if target.notes}<p class="notes"><Icon name="logs" size={12} /> {target.notes}</p>{/if}

        <div class="actions">
          {#if target.auth}
            <button class="btn" onclick={authenticate} title="Opens a terminal to log in (browser auth)">
              <Icon name="github" size={12} /> Authenticate
            </button>
          {/if}
          <button class="btn" onclick={generate} disabled={generating}>
            <Icon name="box" size={12} /> {generating ? 'Generating…' : `Generate ${target.configFile ?? 'config'}`}
          </button>
          {#if target.autoRun}
            <button class="btn primary" onclick={deploy} disabled={deploying || !writtenPath}>
              <Icon name="rocket" size={12} /> {deploying ? 'Deploying…' : 'Deploy'}
            </button>
          {/if}
          {#if commandStr}
            <button class="btn" onclick={openInTerminal} title="Run in a real terminal (answer prompts there)">
              <Icon name="logs" size={12} /> Open in terminal
            </button>
          {/if}
          {#if commandStr}<button class="btn" onclick={copyCmd}><Icon name="copy" size={12} /> Copy</button>{/if}
        </div>
        <p class="auth-hint">Browser-login CLIs (gcloud, vercel, az…) can't take a pasted token — click <strong>Authenticate</strong> to log in once in a real terminal, then Deploy runs headlessly. Or use <strong>Open in terminal</strong> to run any command with prompts.</p>

        {#if genErr}<div class="err-note">{genErr}</div>{/if}
        {#if cliMissing}<div class="err-note">`{target.cli}` CLI not found. Install: <code>{target.install}</code></div>{/if}

        {#if generatedConfig}
          <div class="block">
            <div class="block-h">{writtenPath ?? 'config'}</div>
            <pre class="code">{generatedConfig}</pre>
          </div>
        {/if}

        {#if commandStr}
          <div class="block">
            <div class="block-h">deploy command {target.autoRun ? '' : '(run manually)'}</div>
            <pre class="code cmd">{commandStr}</pre>
          </div>
        {/if}

        {#if target.urlHint}<p class="hint">URL: <code>{target.urlHint}</code></p>{/if}

        {#if logLines.length || deploying}
          <div class="block">
            <div class="block-h">deploy log {exitCode !== null ? `· exit ${exitCode}` : ''}</div>
            <div class="log" bind:this={logEl}>
              {#each logLines as l}<div class="log-line">{l}</div>{/each}
              {#if deploying}<div class="log-line dim">…</div>{/if}
            </div>
          </div>
        {/if}

        {#if deployUrl}
          <div class="url-note">
            <Icon name="check" size={13} /> Live at <button class="link" onclick={visit}>{deployUrl} ↗</button>
          </div>
        {/if}
      </div>
    {:else}
      <p class="pick">Pick a deploy target above. Crush ejects a Dockerfile + the provider's native config, then runs its official CLI.</p>
    {/if}
  </div>
</div>

<style>
  .overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.6); backdrop-filter: blur(2px); z-index: 40; border: none; padding: 0; cursor: default; }
  .modal { position: fixed; top: 3vh; left: 50%; transform: translateX(-50%); width: min(820px, 94vw); max-height: 94vh; background: var(--color-crush-dark); border: 1px solid var(--color-crush-border); border-radius: 0.75rem; z-index: 50; display: flex; flex-direction: column; overflow: hidden; box-shadow: 0 24px 80px rgba(0,0,0,0.6); }
  .m-head { display: flex; align-items: center; justify-content: space-between; padding: 14px 18px; border-bottom: 1px solid var(--color-crush-border); }
  .m-title { display: flex; align-items: center; gap: 8px; }
  .m-title h2 { font-size: 15px; font-weight: 600; margin: 0; }
  .close-btn { background: none; border: none; color: var(--color-crush-text-muted); font-size: 22px; line-height: 1; cursor: pointer; }
  .m-body { padding: 16px 18px; overflow-y: auto; }

  .targets { display: grid; grid-template-columns: repeat(auto-fill, minmax(150px, 1fr)); gap: 8px; margin-bottom: 16px; }
  .target { display: flex; flex-direction: column; gap: 3px; align-items: flex-start; text-align: left; background: var(--color-crush-surface); border: 1px solid var(--color-crush-border); border-radius: 8px; padding: 9px 11px; cursor: pointer; color: var(--color-crush-text); }
  .target:hover { border-color: var(--color-crush-muted); }
  .target.active { border-color: var(--color-crush-primary); background: rgba(255,255,255,0.06); }
  .t-top { display: flex; align-items: center; gap: 8px; }
  .t-top :global(svg) { flex-shrink: 0; }
  .mono { display: inline-flex; align-items: center; justify-content: center; width: 18px; height: 18px; border-radius: 4px; color: #fff; font-size: 8px; font-weight: 700; letter-spacing: -0.02em; flex-shrink: 0; }
  .t-label { font-size: 13px; font-weight: 500; }
  .t-group { font-size: 10px; text-transform: uppercase; letter-spacing: 0.04em; color: var(--color-crush-text-muted); }
  .t-group.registry { color: #c084fc; }
  .t-group.frontend { color: #22d3ee; }
  .t-group.vps { color: #4ade80; }
  .rec-head { display: flex; align-items: baseline; gap: 8px; font-size: 11px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-crush-text-muted); margin-bottom: 8px; }
  .rec-head .kind { color: var(--color-crush-text); font-weight: 600; }
  .rec-head .kind-sub { text-transform: none; letter-spacing: 0; color: var(--color-crush-muted); font-size: 11px; }
  .others-toggle { background: none; border: none; color: var(--color-crush-text-muted); font-size: 12px; cursor: pointer; padding: 6px 0; margin-top: 4px; }
  .others-toggle:hover { color: var(--color-crush-text); }
  .dim-targets { margin-top: 6px; opacity: 0.75; }

  .fields { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; margin-bottom: 12px; }
  .fld { display: flex; flex-direction: column; gap: 4px; }
  .fk { font-size: 11px; color: var(--color-crush-text-muted); }
  .env { font-family: var(--font-mono); font-size: 10px; color: var(--color-crush-muted); }
  .fld input { background: var(--color-crush-surface); border: 1px solid var(--color-crush-border); border-radius: 7px; color: var(--color-crush-text); padding: 7px 9px; font-size: 13px; outline: none; }
  .fld input:focus { border-color: var(--color-crush-muted); }

  .notes { display: flex; align-items: flex-start; gap: 6px; font-size: 12px; color: var(--color-crush-text-muted); background: var(--color-crush-surface); border-radius: 7px; padding: 8px 10px; line-height: 1.5; margin: 0 0 12px; }
  .actions { display: flex; gap: 8px; flex-wrap: wrap; margin-bottom: 8px; }
  .auth-hint { font-size: 11.5px; color: var(--color-crush-text-muted); line-height: 1.5; margin: 0 0 12px; }
  .auth-hint strong { color: var(--color-crush-text); font-weight: 600; }
  .btn { display: inline-flex; align-items: center; gap: 6px; font-size: 12px; color: var(--color-crush-text-muted); background: none; border: 1px solid var(--color-crush-border); border-radius: 7px; padding: 7px 13px; cursor: pointer; }
  .btn:hover:not(:disabled) { color: var(--color-crush-text); border-color: var(--color-crush-muted); }
  .btn:disabled { opacity: 0.45; cursor: default; }
  .btn.primary { color: var(--color-crush-on-primary); background: var(--color-crush-primary); border-color: var(--color-crush-primary); }
  .btn.primary:hover:not(:disabled) { background: var(--color-crush-primary-hover); border-color: var(--color-crush-primary-hover); }

  .err-note { font-size: 12px; color: var(--color-crush-red); background: rgba(239,68,68,0.1); border: 1px solid rgba(239,68,68,0.25); border-radius: 7px; padding: 8px 10px; margin-bottom: 12px; }
  .err-note code { font-family: var(--font-mono); }
  .block { border: 1px solid var(--color-crush-border); border-radius: 8px; overflow: hidden; margin-bottom: 12px; }
  .block-h { font-size: 10px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-crush-text-muted); padding: 7px 10px; background: var(--color-crush-surface); border-bottom: 1px solid var(--color-crush-border); font-family: var(--font-mono); }
  .code { margin: 0; padding: 10px 12px; font-family: var(--font-mono); font-size: 11.5px; line-height: 1.5; color: var(--color-crush-text); white-space: pre-wrap; word-break: break-word; max-height: 220px; overflow-y: auto; background: rgba(9,9,11,0.6); }
  .code.cmd { color: #67e8f9; }
  .hint { font-size: 11.5px; color: var(--color-crush-text-muted); margin: 0 0 12px; }
  .hint code { font-family: var(--font-mono); color: var(--color-crush-text); }
  .log { max-height: 240px; overflow-y: auto; padding: 8px 12px; font-family: var(--font-mono); font-size: 11px; line-height: 1.55; background: rgba(9,9,11,0.85); }
  .log-line { color: var(--color-crush-text); white-space: pre-wrap; word-break: break-word; }
  .log-line.dim { color: var(--color-crush-muted); }
  .url-note { display: flex; align-items: center; gap: 8px; font-size: 13px; color: var(--color-crush-green); background: rgba(16,185,129,0.1); border: 1px solid rgba(16,185,129,0.25); border-radius: 8px; padding: 10px 12px; }
  .link { background: none; border: none; color: var(--color-crush-text); cursor: pointer; font-family: var(--font-mono); font-size: 13px; padding: 0; }
  .link:hover { text-decoration: underline; }
  .pick { color: var(--color-crush-text-muted); font-size: 13px; }
</style>
