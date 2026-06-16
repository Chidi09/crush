<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { run } from '$lib/stores/run.svelte';
  import * as api from '$lib/tauri';
  import type { ApiModel, Group, RequestSpec, ApiSendRequest, ApiResponse, CapturedExample, Header, Param, Variable } from '$lib/tauri';

  // State (Svelte 5 Runes)
  let projectPath = $state('/root/crush');
  let apiModel = $state<ApiModel | null>(null);
  let loading = $state(true);
  let liveUrl = $state('http://localhost:3000');
  
  // Navigation / Selection
  let selectedGroup = $state<Group | null>(null);
  let selectedRequest = $state<RequestSpec | null>(null);
  let searchQuery = $state('');

  // Active view: 'endpoint' | 'guides'
  let activeView = $state<'endpoint' | 'guides'>('endpoint');
  let activeGuide = $state<'auth' | 'pagination' | 'webhooks' | 'custom'>('auth');
  let customGuideTitle = $state('Custom Narrative Guide');
  let customGuideContent = $state('Write markdown docs here...');
  let editGuide = $state(false);

  // Tabs for main request builder
  let activeTab = $state<'params' | 'headers' | 'body' | 'auth' | 'docs' | 'codegen'>('params');

  // Input states for selected request
  let paramValues = $state<Record<string, string>>({});
  let headerValues = $state<Record<string, string>>({});
  let bodyValue = $state('');
  
  let authType = $state<'inherited' | 'bearer' | 'basic' | 'apikey' | 'none'>('inherited');
  let authBearerToken = $state('');
  let authBasicUser = $state('');
  let authBasicPass = $state('');
  let authApiKeyName = $state('');
  let authApiKeyValue = $state('');
  let authApiKeyIn = $state<'header' | 'query'>('header');

  // Sending request status
  let sending = $state(false);
  let response = $state<ApiResponse | null>(null);
  let responseError = $state<string | null>(null);
  let responseTab = $state<'body' | 'headers' | 'raw'>('body');

  // Codegen
  let codegenLang = $state<'curl' | 'fetch'>('curl');

  // Sandbox DB state
  let sandboxUrl = $state<string | null>(null);
  let creatingSandbox = $state(false);

  // Publishing docs
  let publishingDocs = $state(false);
  let publishedPath = $state<string | null>(null);

  // Notebook Responses
  let notebookResponses = $state<Record<string, { status: number; body: string; timing: number; loading: boolean }>>({});

  // Example Saving
  let showSaveExampleModal = $state(false);
  let saveExampleLabel = $state('');
  let saveExampleIsError = $state(false);

  // Discovery / Paste spec UI
  let importContent = $state('');
  let importUrl = $state('');
  let importError = $state<string | null>(null);
  let importing = $state(false);
  let probing = $state(false);

  // Derived state: Filtered groups/requests
  let filteredGroups = $derived(
    apiModel ? apiModel.groups.map(g => {
      const reqs = g.requests.filter(r => 
        r.path.toLowerCase().includes(searchQuery.toLowerCase()) || 
        r.method.toLowerCase().includes(searchQuery.toLowerCase()) || 
        r.doc.summary.toLowerCase().includes(searchQuery.toLowerCase())
      );
      return { ...g, requests: reqs };
    }).filter(g => g.requests.length > 0) : []
  );

  onMount(async () => {
    loading = true;
    try {
      // 1. Resolve project path
      if (run.projectPath) {
        projectPath = run.projectPath;
      } else {
        const saved = localStorage.getItem('crush:lastProject');
        if (saved) projectPath = saved;
      }

      // 2. Resolve default liveUrl
      if (run.endpoints && run.endpoints.length > 0) {
        liveUrl = run.endpoints[0].url;
      }

      // 2.5 Resolve sandbox
      const savedSandbox = localStorage.getItem('crush:sandboxUrl');
      if (savedSandbox) sandboxUrl = savedSandbox;

      // 3. Try to load existing spec from project
      await loadSpec();
    } catch (e) {
      console.error("Failed on mount:", e);
    } finally {
      loading = false;
    }
  });

  function generateCurl(): string {
    if (!selectedRequest) return '';
    let finalUrl = selectedRequest.path;
    if (!finalUrl.startsWith('http://') && !finalUrl.startsWith('https://')) {
      const base = liveUrl.replace(/\/+$/, '');
      finalUrl = `${base}${finalUrl.startsWith('/') ? '' : '/'}${finalUrl}`;
    }
    for (const p of selectedRequest.params) {
      if (p.in_location === 'path') {
        finalUrl = finalUrl.replace(`{${p.name}}`, encodeURIComponent(paramValues[p.name] || ''));
      }
    }
    const queryParams = new URLSearchParams();
    for (const p of selectedRequest.params) {
      if (p.in_location === 'query' && paramValues[p.name]) {
        queryParams.append(p.name, paramValues[p.name]);
      }
    }
    const queryString = queryParams.toString();
    if (queryString) {
      finalUrl += (finalUrl.includes('?') ? '&' : '?') + queryString;
    }

    let curl = `curl -X ${selectedRequest.method} "${finalUrl}"`;
    for (const [k, v] of Object.entries(headerValues)) {
      if (v) curl += ` \\\n  -H "${k}: ${v}"`;
    }
    if (authType === 'bearer' && authBearerToken) {
      curl += ` \\\n  -H "Authorization: Bearer ${authBearerToken}"`;
    } else if (authType === 'basic' && authBasicUser) {
      const credentials = btoa(`${authBasicUser}:${authBasicPass}`);
      curl += ` \\\n  -H "Authorization: Basic ${credentials}"`;
    } else if (authType === 'apikey' && authApiKeyIn === 'header' && authApiKeyName) {
      curl += ` \\\n  -H "${authApiKeyName}: ${authApiKeyValue}"`;
    }
    if (selectedRequest.body && bodyValue) {
      const escapedBody = bodyValue.replace(/'/g, "'\\''");
      curl += ` \\\n  -d '${escapedBody}'`;
    }
    return curl;
  }

  function generateFetch(): string {
    if (!selectedRequest) return '';
    let finalUrl = selectedRequest.path;
    if (!finalUrl.startsWith('http://') && !finalUrl.startsWith('https://')) {
      const base = liveUrl.replace(/\/+$/, '');
      finalUrl = `${base}${finalUrl.startsWith('/') ? '' : '/'}${finalUrl}`;
    }
    for (const p of selectedRequest.params) {
      if (p.in_location === 'path') {
        finalUrl = finalUrl.replace(`{${p.name}}`, encodeURIComponent(paramValues[p.name] || ''));
      }
    }
    const queryParams = new URLSearchParams();
    for (const p of selectedRequest.params) {
      if (p.in_location === 'query' && paramValues[p.name]) {
        queryParams.append(p.name, paramValues[p.name]);
      }
    }
    const queryString = queryParams.toString();
    if (queryString) {
      finalUrl += (finalUrl.includes('?') ? '&' : '?') + queryString;
    }

    const headersObj: Record<string, string> = {};
    for (const [k, v] of Object.entries(headerValues)) {
      if (v) headersObj[k] = v;
    }
    if (authType === 'bearer' && authBearerToken) {
      headersObj['Authorization'] = `Bearer ${authBearerToken}`;
    } else if (authType === 'basic' && authBasicUser) {
      const credentials = btoa(`${authBasicUser}:${authBasicPass}`);
      headersObj['Authorization'] = `Basic ${credentials}`;
    } else if (authType === 'apikey' && authApiKeyIn === 'header' && authApiKeyName) {
      headersObj[authApiKeyName] = authApiKeyValue;
    }

    const fetchOpts = {
      method: selectedRequest.method,
      headers: headersObj,
      body: selectedRequest.body ? bodyValue : undefined,
    };
    return `fetch("${finalUrl}", ${JSON.stringify(fetchOpts, null, 2)});`;
  }

  async function handleCreateSandbox() {
    creatingSandbox = true;
    try {
      const url = await api.dbCreateSandbox(projectPath);
      sandboxUrl = url;
      localStorage.setItem('crush:sandboxUrl', url);
    } catch (e) {
      alert("Failed to create database sandbox: " + e);
    } finally {
      creatingSandbox = false;
    }
  }

  async function handleResetSandbox() {
    if (!sandboxUrl) return;
    const parts = sandboxUrl.split('/crush_sandbox_');
    if (parts.length < 2) return;
    const sandboxId = parts[1];
    loading = true;
    try {
      await api.dbResetSandbox(sandboxId);
    } catch (e) {
      alert("Failed to reset database sandbox: " + e);
    } finally {
      loading = false;
    }
  }

  async function handleDestroySandbox() {
    if (!sandboxUrl) return;
    const parts = sandboxUrl.split('/crush_sandbox_');
    if (parts.length < 2) return;
    const sandboxId = parts[1];
    loading = true;
    try {
      await api.dbDestroySandbox(sandboxId);
      sandboxUrl = null;
      localStorage.removeItem('crush:sandboxUrl');
    } catch (e) {
      alert("Failed to destroy database sandbox: " + e);
    } finally {
      loading = false;
    }
  }

  async function handlePublishDocs() {
    publishingDocs = true;
    publishedPath = null;
    try {
      const path = await api.apiPublishDocs(projectPath);
      publishedPath = path;
    } catch (e) {
      alert("Failed to publish docs: " + e);
    } finally {
      publishingDocs = false;
    }
  }

  function selectRequestById(id: string) {
    if (!apiModel) return;
    for (const group of apiModel.groups) {
      const found = group.requests.find(r => r.id === id);
      if (found) {
        selectRequest(found, group);
        return;
      }
    }
    alert("Referenced endpoint not found: " + id);
  }

  async function runNotebookEndpoint(id: string) {
    if (!apiModel) return;
    let req: RequestSpec | null = null;
    for (const group of apiModel.groups) {
      const found = group.requests.find(r => r.id === id);
      if (found) { req = found; break; }
    }
    if (!req) {
      alert("Endpoint not found for run: " + id);
      return;
    }

    notebookResponses[id] = { status: 0, body: '', timing: 0, loading: true };
    try {
      let finalUrl = req.path;
      if (!finalUrl.startsWith('http://') && !finalUrl.startsWith('https://')) {
        finalUrl = `${liveUrl.replace(/\/+$/, '')}/${finalUrl.replace(/^\/+/, '')}`;
      }

      const headersList: Header[] = [];
      for (const h of req.headers) {
        headersList.push({ name: h.name, value: h.value, description: null });
      }
      if (authType === 'bearer' && authBearerToken) {
        headersList.push({ name: 'Authorization', value: `Bearer ${authBearerToken}`, description: null });
      }

      const body = req.body?.example ? JSON.stringify(req.body.example) : null;

      const resp = await api.apiSend(projectPath, {
        method: req.method,
        url: finalUrl,
        headers: headersList,
        body,
      });

      notebookResponses[id] = {
        status: resp.status,
        body: resp.body || '',
        timing: resp.timing_ms,
        loading: false
      };
    } catch (e) {
      notebookResponses[id] = {
        status: 500,
        body: String(e),
        timing: 0,
        loading: false
      };
    }
  }

  function renderGuideText(text: string) {
    const parts: Array<
      | { type: 'text'; content: string }
      | { type: 'endpoint' | 'run-endpoint'; id: string }
    > = [];
    let lastIndex = 0;
    const regex = /\[(endpoint|run-endpoint):\s*([a-zA-Z0-9_-]+)\]/g;
    let match;
    while ((match = regex.exec(text)) !== null) {
      if (match.index > lastIndex) {
        parts.push({ type: 'text', content: text.substring(lastIndex, match.index) });
      }
      if (match[1] === 'endpoint') {
        parts.push({ type: 'endpoint', id: match[2] ?? '' });
      } else {
        parts.push({ type: 'run-endpoint', id: match[2] ?? '' });
      }
      lastIndex = regex.lastIndex;
    }
    if (lastIndex < text.length) {
      parts.push({ type: 'text', content: text.substring(lastIndex) });
    }
    return parts;
  }

  async function loadSpec() {
    try {
      const model = await api.apiLoadSpec(projectPath);
      if (model) {
        apiModel = model;
        // Auto select first request if any
        if (model.groups.length > 0 && model.groups[0].requests.length > 0) {
          selectRequest(model.groups[0].requests[0], model.groups[0]);
        }
      } else {
        apiModel = null;
      }
    } catch (e) {
      console.error("Failed to load spec:", e);
    }
  }

  function selectRequest(req: RequestSpec, group: Group) {
    selectedRequest = req;
    selectedGroup = group;
    response = null;
    responseError = null;

    // Reset parameters & headers input fields
    paramValues = {};
    for (const p of req.params) {
      paramValues[p.name] = p.schema?.default !== undefined ? String(p.schema.default) : '';
    }

    headerValues = {};
    for (const h of req.headers) {
      headerValues[h.name] = h.value;
    }

    // Set body value
    if (req.body) {
      bodyValue = req.body.example ? JSON.stringify(req.body.example, null, 2) : '';
    } else {
      bodyValue = '';
    }

    // Resolve Auth
    if (req.auth) {
      if (req.auth.type === 'bearer') {
        authType = 'bearer';
        authBearerToken = req.auth.token || '';
      } else if (req.auth.type === 'basic') {
        authType = 'basic';
        authBasicUser = req.auth.username || '';
        authBasicPass = req.auth.password || '';
      } else if (req.auth.type === 'apiKey') {
        authType = 'apikey';
        authApiKeyName = req.auth.key || '';
        authApiKeyValue = req.auth.value || '';
        authApiKeyIn = req.auth.inLocation || 'header';
      }
    } else {
      authType = 'inherited';
    }

    activeView = 'endpoint';
  }

  async function handleImport() {
    if (!importContent.trim()) {
      importError = "Please paste spec content first";
      return;
    }
    importing = true;
    importError = null;
    try {
      const model = await api.apiImportSpec(projectPath, importContent);
      apiModel = model;
      importContent = '';
      if (model.groups.length > 0 && model.groups[0].requests.length > 0) {
        selectRequest(model.groups[0].requests[0], model.groups[0]);
      }
    } catch (e) {
      importError = String(e);
    } finally {
      importing = false;
    }
  }

  async function handleScan() {
    importing = true;
    importError = null;
    try {
      const content = await api.apiScanProject(projectPath);
      if (content) {
        const model = await api.apiImportSpec(projectPath, content);
        apiModel = model;
        if (model.groups.length > 0 && model.groups[0].requests.length > 0) {
          selectRequest(model.groups[0].requests[0], model.groups[0]);
        }
      } else {
        importError = "No spec files found in project root or docs directory.";
      }
    } catch (e) {
      importError = String(e);
    } finally {
      importing = false;
    }
  }

  async function handleProbe() {
    probing = true;
    importError = null;
    try {
      const content = await api.apiProbeLive(liveUrl);
      if (content) {
        const model = await api.apiImportSpec(projectPath, content);
        apiModel = model;
        if (model.groups.length > 0 && model.groups[0].requests.length > 0) {
          selectRequest(model.groups[0].requests[0], model.groups[0]);
        }
      } else {
        importError = `No OpenAPI spec found at ${liveUrl}. Emitting standard FastAPI/Swagger paths failed.`;
      }
    } catch (e) {
      importError = String(e);
    } finally {
      probing = false;
    }
  }

  async function triggerSend() {
    if (!selectedRequest) return;
    sending = true;
    response = null;
    responseError = null;

    try {
      // 1. Build url and replace path parameters
      let finalUrl = selectedRequest.path;
      
      // If it's a relative path, prefix with liveUrl
      if (!finalUrl.startsWith('http://') && !finalUrl.startsWith('https://')) {
        const base = liveUrl.replace(/\/+$/, '');
        finalUrl = `${base}${finalUrl.startsWith('/') ? '' : '/'}${finalUrl}`;
      }

      // Replace path parameters
      for (const p of selectedRequest.params) {
        if (p.in_location === 'path') {
          finalUrl = finalUrl.replace(`{${p.name}}`, encodeURIComponent(paramValues[p.name] || ''));
        }
      }

      // Append query parameters
      const queryParams = new URLSearchParams();
      for (const p of selectedRequest.params) {
        if (p.in_location === 'query' && paramValues[p.name]) {
          queryParams.append(p.name, paramValues[p.name]);
        }
      }
      const queryString = queryParams.toString();
      if (queryString) {
        finalUrl += (finalUrl.includes('?') ? '&' : '?') + queryString;
      }

      // 2. Build headers
      const headersList: Header[] = [];
      for (const [k, v] of Object.entries(headerValues)) {
        if (v) {
          headersList.push({ name: k, value: v, description: null });
        }
      }

      // Add Auth Header if applicable
      if (authType === 'bearer' && authBearerToken) {
        headersList.push({ name: 'Authorization', value: `Bearer ${authBearerToken}`, description: null });
      } else if (authType === 'basic' && authBasicUser) {
        const credentials = btoa(`${authBasicUser}:${authBasicPass}`);
        headersList.push({ name: 'Authorization', value: `Basic ${credentials}`, description: null });
      } else if (authType === 'apikey' && authApiKeyIn === 'header' && authApiKeyName) {
        headersList.push({ name: authApiKeyName, value: authApiKeyValue, description: null });
      } else if (authType === 'inherited' && apiModel?.auth) {
        if (apiModel.auth.type === 'bearer' && apiModel.auth.bearer?.token) {
          headersList.push({ name: 'Authorization', value: `Bearer ${apiModel.auth.bearer.token}`, description: null });
        }
      }

      // 3. Build Body
      const body = selectedRequest.body ? bodyValue : null;

      const resp = await api.apiSend(projectPath, {
        method: selectedRequest.method,
        url: finalUrl,
        headers: headersList,
        body,
      });

      response = resp;
      responseTab = 'body';
    } catch (e) {
      responseError = String(e);
    } finally {
      sending = false;
    }
  }

  function openSaveExampleModal(isError: boolean) {
    saveExampleLabel = response?.status ? `Response ${response.status}` : 'Example';
    saveExampleIsError = isError;
    showSaveExampleModal = true;
  }

  async function saveExample() {
    if (!selectedRequest || !selectedGroup || !response) return;
    try {
      const example: CapturedExample = {
        label: saveExampleLabel,
        request: {
          method: selectedRequest.method,
          url: selectedRequest.path,
          headers: selectedRequest.headers,
          body: selectedRequest.body ? bodyValue : null,
        },
        response: {
          status: response.status,
          headers: response.headers,
          body: response.body,
          timing_ms: response.timing_ms,
          size_bytes: response.size_bytes,
        },
        verified_at: Math.floor(Date.now() / 1000),
        schema_ok: null, // verified_example command will perform schema validation next
      };

      const updated = await api.apiSaveExample(
        projectPath,
        selectedGroup.name,
        selectedRequest.id,
        saveExampleIsError,
        example
      );

      // Verify immediately to check schema
      const verified = await api.apiVerifyExample(
        projectPath,
        selectedGroup.name,
        selectedRequest.id,
        saveExampleLabel,
        saveExampleIsError
      );

      apiModel = verified;
      
      // Update selectedRequest inside the model
      const g = verified.groups.find(x => x.name === selectedGroup!.name);
      if (g) {
        const r = g.requests.find(x => x.id === selectedRequest!.id);
        if (r) selectedRequest = r;
      }
      
      showSaveExampleModal = false;
    } catch (e) {
      alert("Failed to save example: " + e);
    }
  }

  async function verifyExample(ex: CapturedExample, isError: boolean) {
    if (!selectedRequest || !selectedGroup) return;
    try {
      const updated = await api.apiVerifyExample(
        projectPath,
        selectedGroup.name,
        selectedRequest.id,
        ex.label,
        isError
      );
      apiModel = updated;
      const g = updated.groups.find(x => x.name === selectedGroup!.name);
      if (g) {
        const r = g.requests.find(x => x.id === selectedRequest!.id);
        if (r) selectedRequest = r;
      }
    } catch (e) {
      alert("Verification failed: " + e);
    }
  }

  async function verifyAll() {
    if (!apiModel) return;
    loading = true;
    try {
      const updated = await api.apiVerifyAll(projectPath);
      apiModel = updated;
      if (selectedRequest && selectedGroup) {
        const g = updated.groups.find(x => x.name === selectedGroup!.name);
        if (g) {
          const r = g.requests.find(x => x.id === selectedRequest!.id);
          if (r) selectedRequest = r;
        }
      }
    } catch (e) {
      alert("Verify all failed: " + e);
    } finally {
      loading = false;
    }
  }

  function addHeaderRow() {
    headerValues = { ...headerValues, '': '' };
  }

  function removeHeaderRow(key: string) {
    const next = { ...headerValues };
    delete next[key];
    headerValues = next;
  }

  function getMethodColor(method: string): string {
    const m = method.toUpperCase();
    if (m === 'GET') return 'var(--color-crush-primary)';
    if (m === 'POST') return 'var(--color-crush-warning)';
    if (m === 'PUT') return 'var(--color-crush-info)';
    if (m === 'DELETE') return 'var(--color-crush-danger)';
    return 'var(--color-crush-text-muted)';
  }

  function formatTime(unixSecs: number): string {
    if (unixSecs === 0) return 'never';
    const diff = Math.floor(Date.now() / 1000) - unixSecs;
    if (diff < 60) return 'just now';
    if (diff < 3600) return `${Math.floor(diff / 60)} min ago`;
    return `${Math.floor(diff / 3600)} hours ago`;
  }
</script>

<div class="api-studio">
  <!-- Left Side: Endpoint Sidebar -->
  <div class="api-sidebar">
    <div class="sidebar-header">
      <div class="title-row">
        <h3>API Studio</h3>
        {#if apiModel}
          <button class="btn icon-btn" title="Verify all examples" onclick={verifyAll}>
            <Icon name="refresh" size={14} />
          </button>
        {/if}
      </div>
      <input
        type="text"
        placeholder="Filter endpoints..."
        bind:value={searchQuery}
        class="search-input"
      />
      
      <!-- Database Sandbox Widget (Pillars D1/D4) -->
      <div class="sandbox-banner">
        <div class="sandbox-info">
          <Icon name="database" size={14} />
          {#if sandboxUrl}
            <span class="status-active">Sandbox Active</span>
          {:else}
            <span class="status-none">No Sandbox DB</span>
          {/if}
        </div>
        <div class="sandbox-actions">
          {#if sandboxUrl}
            <button class="btn icon-btn mini" title="Reset Sandbox to pristine" onclick={handleResetSandbox}>
              <Icon name="refresh" size={12} />
            </button>
            <button class="btn icon-btn mini danger" title="Destroy Sandbox" onclick={handleDestroySandbox}>
              <Icon name="trash" size={12} />
            </button>
          {:else}
            <button class="btn primary mini" onclick={handleCreateSandbox} disabled={creatingSandbox}>
              {#if creatingSandbox}Spawning...{:else}Create Sandbox{/if}
            </button>
          {/if}
        </div>
      </div>
    </div>

    <div class="sidebar-content">
      {#if loading}
        <div class="loading-state">Loading studio...</div>
      {:else if !apiModel}
        <div class="empty-sidebar">
          <p>No spec loaded.</p>
        </div>
      {:else}
        <!-- Navigation Guides Link -->
        <button
          class="guide-nav-item"
          class:active={activeView === 'guides'}
          onclick={() => activeView = 'guides'}
        >
          <Icon name="docs" size={16} />
          <span>Narrative Guides</span>
        </button>

        <div class="divider"></div>

        <!-- Groups & Requests Tree -->
        {#each filteredGroups as group}
          <div class="group-section">
            <div class="group-header">
              <Icon name="folder" size={14} />
              <span>{group.name}</span>
            </div>
            <div class="group-requests">
              {#each group.requests as req}
                <button
                  class="request-item"
                  class:active={activeView === 'endpoint' && selectedRequest?.id === req.id}
                  onclick={() => selectRequest(req, group)}
                >
                  <span class="method-badge" style="color: {getMethodColor(req.method)}">{req.method}</span>
                  <span class="path-text" title={req.path}>{req.doc.summary || req.path}</span>
                </button>
              {/each}
            </div>
          </div>
        {/each}
      {/if}
    </div>
  </div>

  <!-- Main View Area (Center + Right) -->
  <div class="api-main-content">
    {#if !apiModel && !loading}
      <!-- Empty discovery UI -->
      <div class="discovery-container">
        <div class="discovery-card">
          <div class="card-icon">
            <Icon name="api" size={48} />
          </div>
          <h2>Connect API Studio</h2>
          <p class="subtitle">
            Zero-config API exploration and documentation deeply correlated with your running stack.
          </p>

          <div class="discovery-actions">
            <div class="action-box">
              <h4>Scan Codebase</h4>
              <p>Search for swagger.json or openapi.yaml in your project directory.</p>
              <button class="btn primary" onclick={handleScan} disabled={importing}>
                {#if importing}Scanning...{:else}Scan Project{/if}
              </button>
            </div>

            <div class="action-box">
              <h4>Probe Live App</h4>
              <p>Hit common OpenAPI endpoints on your running application.</p>
              <div class="url-input-row">
                <input type="text" bind:value={liveUrl} placeholder="http://localhost:3000" />
                <button class="btn success" onclick={handleProbe} disabled={probing}>
                  {#if probing}Probing...{:else}Probe Live{/if}
                </button>
              </div>
            </div>
          </div>

          <div class="paste-section">
            <h4>Or paste OpenAPI / Postman Spec (JSON/YAML)</h4>
            <textarea
              bind:value={importContent}
              placeholder="Paste raw JSON or YAML schema here..."
              rows={8}
            ></textarea>
            {#if importError}
              <div class="error-msg">{importError}</div>
            {/if}
            <button class="btn secondary" onclick={handleImport} disabled={importing}>
              Import Spec
            </button>
          </div>
        </div>
      </div>
    {:else if activeView === 'guides'}
      <!-- Narrative Guides View (Pillar C3/C4) -->
      <div class="guides-container">
        <div class="guides-sidebar">
          <h4>Narrative Guides</h4>
          <button class="guide-link" class:active={activeGuide === 'auth'} onclick={() => activeGuide = 'auth'}>Authentication</button>
          <button class="guide-link" class:active={activeGuide === 'pagination'} onclick={() => activeGuide = 'pagination'}>Pagination & Limits</button>
          <button class="guide-link" class:active={activeGuide === 'webhooks'} onclick={() => activeGuide = 'webhooks'}>Webhooks Integration</button>
          <button class="guide-link" class:active={activeGuide === 'custom'} onclick={() => activeGuide = 'custom'}>{customGuideTitle}</button>
        </div>

        <div class="guide-content-pane">
          {#if editGuide}
            <div class="guide-edit-mode">
              <input type="text" bind:value={customGuideTitle} class="guide-title-input" />
              <textarea bind:value={customGuideContent} rows={20} class="guide-textarea"></textarea>
              <div class="guide-edit-actions">
                <button class="btn success" onclick={() => editGuide = false}>Save Guide</button>
              </div>
            </div>
          {:else}
            <div class="guide-view-mode">
              <div class="guide-header-row">
                <h2>{activeGuide === 'custom' ? customGuideTitle : activeGuide.toUpperCase()}</h2>
                <div class="guide-header-actions" style="display: flex; gap: 8px;">
                  <button class="btn secondary" onclick={() => editGuide = true}>Edit Guide</button>
                  <button class="btn success" onclick={handlePublishDocs} disabled={publishingDocs}>
                    {#if publishingDocs}Publishing...{:else}Publish Docs{/if}
                  </button>
                </div>
              </div>

              {#if publishedPath}
                <div class="published-banner">
                  <Icon name="check" size={14} />
                  <span>Docs published to <code>{publishedPath}</code></span>
                  <button class="btn secondary mini" onclick={() => api.openUrl('file://' + publishedPath)}>Open in Browser</button>
                </div>
              {/if}

              <div class="markdown-body">
                {#if activeGuide === 'auth'}
                  <p>Our API uses Bearer token authentication. Ensure you resolve the <code>Authorization: Bearer &lt;token&gt;</code> header on all requests.</p>
                  <blockquote>
                    <p><strong>Variables Mapping:</strong> You can define <code>API_TOKEN</code> inside your project <code>.env</code> file. Crush will automatically swap <code>&#123;&#123;API_TOKEN&#125;&#125;</code> placeholders in live executions.</p>
                  </blockquote>
                {:else if activeGuide === 'pagination'}
                  <p>All list endpoints support cursor-based pagination. Use the <code>cursor</code> query parameter to fetch the next set of items.</p>
                  <pre><code>GET /v1/items?limit=50&amp;cursor=xyz123</code></pre>
                {:else if activeGuide === 'webhooks'}
                  <p>Stripe webhooks should point at the Crush L7 tunnel. See active tunnels in the Tunnels tab and map webhook requests recursively.</p>
                {:else}
                  <div class="custom-guide-rendered">
                    {#each renderGuideText(customGuideContent) as part}
                      {#if part.type === 'text'}
                        <span style="white-space: pre-wrap;">{part.content}</span>
                      {:else if part.type === 'endpoint'}
                        <button class="endpoint-inline-link" onclick={() => selectRequestById(part.id)}>
                          <Icon name="api" size={12} />
                          <span>{part.id}</span>
                        </button>
                      {:else if part.type === 'run-endpoint'}
                        <div class="notebook-cell">
                          <div class="notebook-cell-header">
                            <span class="cell-label">Runnable Block: {part.id}</span>
                            <button class="btn success mini" onclick={() => runNotebookEndpoint(part.id)} disabled={notebookResponses[part.id]?.loading}>
                              {#if notebookResponses[part.id]?.loading}Running...{:else}Run Block <Icon name="play" size={10} fill />{/if}
                            </button>
                          </div>
                          {#if notebookResponses[part.id]}
                            <div class="notebook-cell-results">
                              <div class="result-meta" style="display: flex; gap: 12px; margin-bottom: 8px; font-size: 12px;">
                                <span class="status-badge" class:success={notebookResponses[part.id].status >= 200 && notebookResponses[part.id].status < 300} class:error={notebookResponses[part.id].status >= 400 || notebookResponses[part.id].status === 0}>
                                  Status: {notebookResponses[part.id].status}
                                </span>
                                <span class="timing">{notebookResponses[part.id].timing} ms</span>
                              </div>
                              {#if notebookResponses[part.id].body}
                                <pre class="result-body" style="background: rgba(0,0,0,0.3); padding: 8px; border-radius: 4px; font-size: 11px; max-height: 120px; overflow-y: auto;"><code>{notebookResponses[part.id].body}</code></pre>
                              {/if}
                            </div>
                          {/if}
                        </div>
                      {/if}
                    {/each}
                  </div>
                {/if}
              </div>
            </div>
          {/if}
        </div>
      </div>
    {:else if selectedRequest}
      <!-- Left part of Main content: Request Builder -->
      <div class="request-builder-pane">
        <div class="request-header-card">
          <div class="request-url-bar">
            <span class="method-tag" style="background: {getMethodColor(selectedRequest.method)}15; color: {getMethodColor(selectedRequest.method)}">
              {selectedRequest.method}
            </span>
            <span class="url-path">{selectedRequest.path}</span>
            <button class="btn success send-btn" onclick={triggerSend} disabled={sending}>
              {#if sending}Sending...{:else}Send <Icon name="play" size={14} fill />{/if}
            </button>
          </div>
          {#if selectedRequest.doc.summary}
            <h4 class="request-summary">{selectedRequest.doc.summary}</h4>
          {/if}
        </div>

        <!-- Request Tabs -->
        <div class="tab-header">
          <button class="tab-btn" class:active={activeTab === 'params'} onclick={() => activeTab = 'params'}>Parameters ({selectedRequest.params.length})</button>
          <button class="tab-btn" class:active={activeTab === 'headers'} onclick={() => activeTab = 'headers'}>Headers ({Object.keys(headerValues).length})</button>
          <button class="tab-btn" class:active={activeTab === 'body'} onclick={() => activeTab = 'body'}>Body</button>
          <button class="tab-btn" class:active={activeTab === 'auth'} onclick={() => activeTab = 'auth'}>Auth</button>
          <button class="tab-btn" class:active={activeTab === 'docs'} onclick={() => activeTab = 'docs'}>Saved Examples ({selectedRequest.doc.examples.length + selectedRequest.doc.error_examples.length})</button>
          <button class="tab-btn" class:active={activeTab === 'codegen'} onclick={() => activeTab = 'codegen'}>Code Snippet</button>
        </div>

        <div class="tab-content">
          {#if activeTab === 'params'}
            <div class="params-tab">
              {#if selectedRequest.params.length === 0}
                <div class="empty-tab-state">No parameters for this endpoint.</div>
              {:else}
                <table class="params-table">
                  <thead>
                    <tr>
                      <th>Name</th>
                      <th>Location</th>
                      <th>Required</th>
                      <th>Value</th>
                    </tr>
                  </thead>
                  <tbody>
                    {#each selectedRequest.params as param}
                      <tr>
                        <td>
                          <span class="param-name">{param.name}</span>
                          {#if param.description}
                            <div class="param-desc">{param.description}</div>
                          {/if}
                        </td>
                        <td><span class="location-badge">{param.in_location}</span></td>
                        <td>{param.required ? '✓' : ''}</td>
                        <td>
                          <input
                            type="text"
                            placeholder={param.schema?.type || 'value'}
                            bind:value={paramValues[param.name]}
                            class="param-input"
                          />
                        </td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              {/if}
            </div>
          {:else if activeTab === 'headers'}
            <div class="headers-tab">
              <table class="params-table">
                <thead>
                  <tr>
                    <th>Header Key</th>
                    <th>Value</th>
                    <th></th>
                  </tr>
                </thead>
                <tbody>
                  {#each Object.entries(headerValues) as [k, v]}
                    <tr>
                      <td>
                        <input
                          type="text"
                          value={k}
                          onchange={(e) => {
                            const target = e.target as HTMLInputElement;
                            const next = { ...headerValues };
                            delete next[k];
                            next[target.value] = v;
                            headerValues = next;
                          }}
                          placeholder="Header-Name"
                          class="param-input"
                        />
                      </td>
                      <td>
                        <input
                          type="text"
                          value={v}
                          onchange={(e) => {
                            const target = e.target as HTMLInputElement;
                            headerValues = { ...headerValues, [k]: target.value };
                          }}
                          placeholder="value"
                          class="param-input"
                        />
                      </td>
                      <td>
                        <button class="btn icon-btn danger" onclick={() => removeHeaderRow(k)}>
                          <Icon name="trash" size={14} />
                        </button>
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
              <button class="btn secondary" onclick={addHeaderRow}>+ Add Header</button>
            </div>
          {:else if activeTab === 'body'}
            <div class="body-tab">
              {#if !selectedRequest.body}
                <div class="empty-tab-state">This request does not support a body payload.</div>
              {:else}
                <div class="body-header">
                  <span>Mime Type: <strong>{selectedRequest.body.mime_type}</strong></span>
                  {#if selectedRequest.body.example}
                    <button class="btn secondary btn-sm" onclick={() => bodyValue = JSON.stringify(selectedRequest?.body?.example, null, 2)}>
                      Fill Example
                    </button>
                  {/if}
                </div>
                <textarea
                  bind:value={bodyValue}
                  placeholder="Raw payload string..."
                  rows={14}
                  class="body-editor"
                ></textarea>
                {#if selectedRequest.body.schema}
                  <div class="schema-info">
                    <h5>Body Schema</h5>
                    <pre class="schema-pre">{JSON.stringify(selectedRequest.body.schema, null, 2)}</pre>
                  </div>
                {/if}
              {/if}
            </div>
          {:else if activeTab === 'auth'}
            <div class="auth-tab">
              <label class="form-label" for="auth-scheme-type">Auth Scheme Type</label>
              <select id="auth-scheme-type" bind:value={authType} class="auth-select">
                <option value="inherited">Inherit from Spec Global Auth</option>
                <option value="bearer">Bearer Token</option>
                <option value="basic">Basic Auth</option>
                <option value="apikey">API Key Header/Query</option>
                <option value="none">No Authentication</option>
              </select>

              {#if authType === 'inherited'}
                <div class="inherited-auth-box">
                  {#if apiModel?.auth}
                    <p>Inherits global security scheme: <strong>{apiModel.auth.type}</strong></p>
                  {:else}
                    <p>No global security scheme defined in spec.</p>
                  {/if}
                </div>
              {:else if authType === 'bearer'}
                <div class="auth-fields">
                  <label for="auth-bearer-token">Token</label>
                  <input id="auth-bearer-token" type="password" bind:value={authBearerToken} placeholder="Bearer token value" />
                </div>
              {:else if authType === 'basic'}
                <div class="auth-fields">
                  <label for="auth-basic-user">Username</label>
                  <input id="auth-basic-user" type="text" bind:value={authBasicUser} placeholder="username" />
                  <label for="auth-basic-pass">Password</label>
                  <input id="auth-basic-pass" type="password" bind:value={authBasicPass} placeholder="password" />
                </div>
              {:else if authType === 'apikey'}
                <div class="auth-fields">
                  <label for="auth-apikey-name">Key Name</label>
                  <input id="auth-apikey-name" type="text" bind:value={authApiKeyName} placeholder="X-API-Key" />
                  <label for="auth-apikey-value">Value</label>
                  <input id="auth-apikey-value" type="password" bind:value={authApiKeyValue} placeholder="api key value" />
                  <label for="auth-apikey-in">Location</label>
                  <select id="auth-apikey-in" bind:value={authApiKeyIn}>
                    <option value="header">Header</option>
                    <option value="query">Query</option>
                  </select>
                </div>
              {/if}
            </div>
          {:else if activeTab === 'docs'}
            <div class="docs-tab">
              <h4>Documented Examples (Pillars C1/C2)</h4>
              
              <div class="examples-list">
                {#if selectedRequest.doc.examples.length === 0 && selectedRequest.doc.error_examples.length === 0}
                  <div class="empty-tab-state">No saved examples yet. Send a request to capture one.</div>
                {/if}

                {#each selectedRequest.doc.examples as ex}
                  <div class="example-card">
                    <div class="example-card-header">
                      <h5>{ex.label} <span class="status-indicator success">Success</span></h5>
                      <div class="example-actions">
                        {#if ex.schema_ok === true}
                          <span class="badge success" title="Response conforms to schema">✓ Schema OK</span>
                        {:else if ex.schema_ok === false}
                          <span class="badge danger" title="Response shape drifted from OpenAPI spec">⚠ Schema Drifted</span>
                        {/if}
                        <span class="badge info">{formatTime(ex.verified_at)}</span>
                        <button class="btn btn-sm secondary" onclick={() => verifyExample(ex, false)}>Verify</button>
                      </div>
                    </div>
                    <pre class="example-body">{ex.response.body || "(empty response body)"}</pre>
                  </div>
                {/each}

                {#each selectedRequest.doc.error_examples as ex}
                  <div class="example-card">
                    <div class="example-card-header">
                      <h5>{ex.label} <span class="status-indicator error">Error</span></h5>
                      <div class="example-actions">
                        <span class="badge info">{formatTime(ex.verified_at)}</span>
                        <button class="btn btn-sm secondary" onclick={() => verifyExample(ex, true)}>Verify</button>
                      </div>
                    </div>
                    <pre class="example-body">{ex.response.body || "(empty response body)"}</pre>
                  </div>
                {/each}
              </div>
            </div>
          {:else if activeTab === 'codegen'}
            <div class="codegen-tab">
              <div class="codegen-header">
                <select bind:value={codegenLang} class="codegen-select">
                  <option value="curl">cURL Command</option>
                  <option value="fetch">JS Fetch</option>
                </select>
                <button class="btn secondary mini" onclick={() => navigator.clipboard.writeText(codegenLang === 'curl' ? generateCurl() : generateFetch())}>
                  Copy Code
                </button>
              </div>
              <pre class="codegen-block"><code>{codegenLang === 'curl' ? generateCurl() : generateFetch()}</code></pre>
            </div>
          {/if}
        </div>
      </div>

      <!-- Right part of Main content: Response Viewer -->
      <div class="response-viewer-pane">
        <div class="pane-header">
          <h3>Response</h3>
          {#if response}
            <div class="response-meta">
              <span class="status-badge" class:success={response.status < 400} class:error={response.status >= 400}>
                {response.status}
              </span>
              <span class="time-badge">{response.timing_ms} ms</span>
              <span class="size-badge">{(response.size_bytes / 1024).toFixed(2)} KB</span>
            </div>
          {/if}
        </div>

        {#if responseError}
          <div class="error-container">
            <h4>Connection Failed</h4>
            <p>{responseError}</p>
          </div>
        {:else if !response}
          <div class="empty-response-state">
            <Icon name="play" size={24} />
            <p>Send a request to see the response status, headers, and payload.</p>
          </div>
        {:else}
          <div class="response-actions-row">
            <button class="btn btn-sm success" onclick={() => openSaveExampleModal(false)}>Save Example</button>
            {#if response.status >= 400}
              <button class="btn btn-sm secondary" onclick={() => openSaveExampleModal(true)}>Save Error Example</button>
            {/if}
          </div>

          <div class="tab-header">
            <button class="tab-btn" class:active={responseTab === 'body'} onclick={() => responseTab = 'body'}>Body</button>
            <button class="tab-btn" class:active={responseTab === 'headers'} onclick={() => responseTab = 'headers'}>Headers ({response.headers.length})</button>
          </div>

          <div class="response-tab-content">
            {#if responseTab === 'body'}
              <pre class="response-body-pre">{response.body || "(empty response body)"}</pre>
            {:else if responseTab === 'headers'}
              <table class="params-table">
                <thead>
                  <tr>
                    <th>Header Name</th>
                    <th>Value</th>
                  </tr>
                </thead>
                <tbody>
                  {#each response.headers as h}
                    <tr>
                      <td class="mono">{h.name}</td>
                      <td class="mono">{h.value}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {/if}
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>

<!-- Modal: Save Example -->
{#if showSaveExampleModal}
  <div class="modal-backdrop">
    <div class="modal-card">
      <h3>Save Documented Example</h3>
      <p>Save this request/response snapshot into your repo docs.</p>
      
      <div class="modal-form">
        <label for="save-example-label">Example Label</label>
        <input id="save-example-label" type="text" bind:value={saveExampleLabel} placeholder="Success Case" />
      </div>

      <div class="modal-actions">
        <button class="btn secondary" onclick={() => showSaveExampleModal = false}>Cancel</button>
        <button class="btn success" onclick={saveExample}>Save Example</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .api-studio {
    display: flex;
    width: 100%;
    height: 100vh;
    background: var(--color-crush-dark);
    color: var(--color-crush-text);
  }

  .api-sidebar {
    width: 280px;
    height: 100%;
    border-right: 1px solid var(--color-crush-border);
    display: flex;
    flex-direction: column;
    background: var(--color-crush-dark);
  }

  .sidebar-header {
    padding: 16px;
    border-bottom: 1px solid var(--color-crush-border);
  }

  .title-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
  }

  .title-row h3 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
  }

  .search-input {
    width: 100%;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid var(--color-crush-border);
    border-radius: 6px;
    padding: 8px 12px;
    color: var(--color-crush-text);
    font-size: 13px;
  }

  .sidebar-content {
    flex: 1;
    overflow-y: auto;
    padding: 12px 8px;
  }

  .guide-nav-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    background: transparent;
    border: none;
    padding: 8px 12px;
    border-radius: 6px;
    color: var(--color-crush-text-muted);
    cursor: pointer;
    font-size: 13px;
    text-align: left;
    transition: all 0.2s;
  }

  .guide-nav-item:hover, .guide-nav-item.active {
    background: rgba(255, 255, 255, 0.05);
    color: var(--color-crush-text);
  }

  .divider {
    height: 1px;
    background: var(--color-crush-border);
    margin: 12px 0;
  }

  .group-section {
    margin-bottom: 16px;
  }

  .group-header {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--color-crush-text-muted);
    padding: 4px 8px;
    text-transform: uppercase;
  }

  .group-requests {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-top: 4px;
  }

  .request-item {
    display: flex;
    align-items: center;
    gap: 8px;
    background: transparent;
    border: none;
    border-radius: 6px;
    padding: 6px 12px;
    cursor: pointer;
    text-align: left;
    font-size: 13px;
    width: 100%;
  }

  .request-item:hover, .request-item.active {
    background: rgba(255, 255, 255, 0.05);
  }

  .request-item.active {
    background: rgba(255, 255, 255, 0.1);
    color: white;
  }

  .method-badge {
    font-size: 11px;
    font-weight: 700;
    width: 42px;
    text-align: right;
  }

  .path-text {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--color-crush-text-muted);
  }

  .request-item.active .path-text {
    color: white;
  }

  .api-main-content {
    flex: 1;
    display: flex;
    height: 100%;
    overflow: hidden;
  }

  .discovery-container {
    flex: 1;
    display: flex;
    justify-content: center;
    align-items: center;
    padding: 40px;
    overflow-y: auto;
  }

  .discovery-card {
    max-width: 680px;
    width: 100%;
    background: var(--color-crush-surface);
    border: 1px solid var(--color-crush-border);
    border-radius: 12px;
    padding: 32px;
    text-align: center;
  }

  .card-icon {
    color: var(--color-crush-primary);
    margin-bottom: 16px;
  }

  .discovery-card h2 {
    margin: 0 0 8px 0;
    font-size: 24px;
    font-weight: 600;
  }

  .subtitle {
    color: var(--color-crush-text-muted);
    margin-bottom: 32px;
  }

  .discovery-actions {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 20px;
    margin-bottom: 32px;
    text-align: left;
  }

  .action-box {
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid var(--color-crush-border);
    border-radius: 8px;
    padding: 20px;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
  }

  .action-box h4 {
    margin: 0 0 8px 0;
    font-size: 15px;
  }

  .action-box p {
    color: var(--color-crush-text-muted);
    font-size: 13px;
    margin: 0 0 16px 0;
  }

  .url-input-row {
    display: flex;
    gap: 8px;
  }

  .url-input-row input {
    flex: 1;
    background: rgba(0,0,0,0.2);
    border: 1px solid var(--color-crush-border);
    border-radius: 6px;
    padding: 6px 12px;
    color: white;
    font-size: 13px;
  }

  .paste-section {
    text-align: left;
    border-top: 1px solid var(--color-crush-border);
    padding-top: 24px;
  }

  .paste-section h4 {
    margin: 0 0 12px 0;
  }

  .paste-section textarea {
    width: 100%;
    background: rgba(0,0,0,0.2);
    border: 1px solid var(--color-crush-border);
    border-radius: 6px;
    padding: 12px;
    color: white;
    font-family: monospace;
    font-size: 13px;
    margin-bottom: 12px;
  }

  .request-builder-pane {
    flex: 1.2;
    border-right: 1px solid var(--color-crush-border);
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .request-header-card {
    padding: 16px;
    border-bottom: 1px solid var(--color-crush-border);
  }

  .request-url-bar {
    display: flex;
    align-items: center;
    gap: 12px;
    background: rgba(0, 0, 0, 0.2);
    border: 1px solid var(--color-crush-border);
    border-radius: 8px;
    padding: 8px 12px;
  }

  .method-tag {
    font-weight: 700;
    font-size: 12px;
    padding: 2px 8px;
    border-radius: 4px;
  }

  .url-path {
    flex: 1;
    font-family: monospace;
    font-size: 14px;
  }

  .request-summary {
    margin: 12px 0 0 0;
    font-size: 14px;
    color: var(--color-crush-text-muted);
  }

  .tab-header {
    display: flex;
    background: rgba(0, 0, 0, 0.1);
    border-bottom: 1px solid var(--color-crush-border);
  }

  .tab-btn {
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--color-crush-text-muted);
    cursor: pointer;
    font-size: 13px;
    padding: 12px 16px;
    transition: all 0.2s;
  }

  .tab-btn:hover, .tab-btn.active {
    color: var(--color-crush-text);
  }

  .tab-btn.active {
    border-bottom-color: var(--color-crush-primary);
  }

  .tab-content {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
  }

  .params-table {
    width: 100%;
    border-collapse: collapse;
    margin-bottom: 20px;
  }

  .params-table th, .params-table td {
    padding: 10px 12px;
    text-align: left;
    border-bottom: 1px solid var(--color-crush-border);
  }

  .params-table th {
    font-size: 12px;
    color: var(--color-crush-text-muted);
    font-weight: 600;
  }

  .param-name {
    font-weight: 600;
  }

  .param-desc {
    font-size: 11px;
    color: var(--color-crush-text-muted);
    margin-top: 2px;
  }

  .location-badge {
    font-size: 11px;
    background: rgba(255,255,255,0.05);
    padding: 2px 6px;
    border-radius: 4px;
    text-transform: uppercase;
  }

  .param-input {
    width: 100%;
    background: rgba(0,0,0,0.2);
    border: 1px solid var(--color-crush-border);
    border-radius: 4px;
    padding: 6px 10px;
    color: white;
    font-size: 13px;
  }

  .body-editor {
    width: 100%;
    background: rgba(0,0,0,0.2);
    border: 1px solid var(--color-crush-border);
    border-radius: 6px;
    padding: 12px;
    color: white;
    font-family: monospace;
    font-size: 13px;
  }

  .schema-info {
    margin-top: 20px;
    background: rgba(255,255,255,0.02);
    border: 1px solid var(--color-crush-border);
    border-radius: 6px;
    padding: 16px;
  }

  .schema-pre {
    font-size: 11px;
    color: var(--color-crush-text-muted);
    margin: 8px 0 0 0;
  }

  .response-viewer-pane {
    flex: 1;
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .pane-header {
    padding: 16px;
    border-bottom: 1px solid var(--color-crush-border);
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .pane-header h3 {
    margin: 0;
    font-size: 15px;
  }

  .response-meta {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .status-badge {
    font-weight: 700;
    font-size: 12px;
    padding: 4px 8px;
    border-radius: 4px;
  }

  .status-badge.success {
    background: rgba(76, 175, 80, 0.15);
    color: #4caf50;
  }

  .status-badge.error {
    background: rgba(244, 67, 54, 0.15);
    color: #f44336;
  }

  .time-badge, .size-badge {
    font-size: 12px;
    color: var(--color-crush-text-muted);
  }

  .response-actions-row {
    padding: 12px 16px;
    display: flex;
    gap: 8px;
    border-bottom: 1px solid var(--color-crush-border);
  }

  .response-tab-content {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
  }

  .response-body-pre {
    font-family: monospace;
    font-size: 13px;
    white-space: pre-wrap;
    word-break: break-all;
    margin: 0;
  }

  .empty-response-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    color: var(--color-crush-text-muted);
    padding: 40px;
    text-align: center;
  }

  .empty-response-state p {
    font-size: 13px;
    max-width: 240px;
    margin-top: 12px;
  }

  .guides-container {
    flex: 1;
    display: flex;
    height: 100%;
  }

  .guides-sidebar {
    width: 220px;
    border-right: 1px solid var(--color-crush-border);
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .guide-link {
    background: transparent;
    border: none;
    border-radius: 6px;
    padding: 8px 12px;
    cursor: pointer;
    text-align: left;
    font-size: 13px;
    color: var(--color-crush-text-muted);
  }

  .guide-link.active, .guide-link:hover {
    background: rgba(255,255,255,0.05);
    color: white;
  }

  .guide-content-pane {
    flex: 1;
    overflow-y: auto;
    padding: 32px;
  }

  .guide-header-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 24px;
  }

  .markdown-body {
    font-size: 14px;
    line-height: 1.6;
  }

  .markdown-body blockquote {
    border-left: 4px solid var(--color-crush-primary);
    background: rgba(255, 255, 255, 0.02);
    margin: 16px 0;
    padding: 12px 16px;
  }

  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background: rgba(0,0,0,0.6);
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 100;
  }

  .modal-card {
    background: var(--color-crush-surface);
    border: 1px solid var(--color-crush-border);
    border-radius: 12px;
    padding: 24px;
    max-width: 400px;
    width: 100%;
  }

  .modal-form {
    margin: 16px 0 24px 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .modal-form label {
    font-size: 12px;
    color: var(--color-crush-text-muted);
  }

  .modal-form input {
    background: rgba(0,0,0,0.2);
    border: 1px solid var(--color-crush-border);
    border-radius: 6px;
    padding: 8px 12px;
    color: white;
    font-size: 14px;
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  /* Core UI utility styles (matching Svelte theme) */
  .btn {
    border-radius: 6px;
    padding: 6px 14px;
    font-size: 13px;
    font-weight: 500;
    border: 1px solid transparent;
    cursor: pointer;
    transition: all 0.15s;
  }

  .btn.primary {
    background: var(--color-crush-primary);
    color: white;
  }

  .btn.secondary {
    background: transparent;
    border-color: var(--color-crush-border);
    color: var(--color-crush-text);
  }

  .btn.secondary:hover {
    background: rgba(255,255,255,0.05);
  }

  .btn.success {
    background: #4caf50;
    color: white;
  }

  .btn.danger {
    background: #f44336;
    color: white;
  }

  .icon-btn {
    padding: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border-color: var(--color-crush-border);
    color: var(--color-crush-text-muted);
  }

  .icon-btn:hover {
    color: white;
    background: rgba(255,255,255,0.05);
  }

  .badge {
    font-size: 11px;
    font-weight: 600;
    padding: 2px 6px;
    border-radius: 4px;
  }

  .badge.success {
    background: rgba(76, 175, 80, 0.1);
    color: #4caf50;
    border: 1px solid rgba(76,175,80,0.2);
  }

  .badge.danger {
    background: rgba(244, 67, 54, 0.1);
    color: #f44336;
    border: 1px solid rgba(244,67,54,0.2);
  }

  .badge.info {
    background: rgba(255,255,255,0.05);
    color: var(--color-crush-text-muted);
    border: 1px solid var(--color-crush-border);
  }

  .example-card {
    background: rgba(255,255,255,0.01);
    border: 1px solid var(--color-crush-border);
    border-radius: 8px;
    margin-bottom: 12px;
    overflow: hidden;
  }

  .example-card-header {
    padding: 12px 16px;
    background: rgba(0,0,0,0.1);
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .example-card-header h5 {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
  }

  .example-actions {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .example-body {
    font-family: monospace;
    font-size: 12px;
    padding: 16px;
    background: rgba(0,0,0,0.15);
    max-height: 200px;
    overflow-y: auto;
    margin: 0;
    white-space: pre-wrap;
    word-break: break-all;
  }

  .status-indicator {
    font-size: 11px;
    padding: 2px 6px;
    border-radius: 4px;
    margin-left: 8px;
  }

  .status-indicator.success {
    background: rgba(76,175,80,0.2);
    color: #4caf50;
  }

  .status-indicator.error {
    background: rgba(244,67,54,0.2);
    color: #f44336;
  }
  .sandbox-banner {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 12px;
    background: rgba(255, 255, 255, 0.02);
    border-top: 1px solid var(--color-crush-border);
    border-bottom: 1px solid var(--color-crush-border);
    margin-top: 8px;
  }

  .sandbox-info {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
  }

  .status-active {
    color: #4caf50;
    font-weight: 600;
  }

  .status-none {
    color: var(--color-crush-text-muted);
  }

  .sandbox-actions {
    display: flex;
    gap: 4px;
  }

  .btn.mini {
    padding: 2px 8px;
    font-size: 11px;
    border-radius: 4px;
  }

  .published-banner {
    display: flex;
    align-items: center;
    gap: 12px;
    background: rgba(76, 175, 80, 0.1);
    border: 1px solid rgba(76, 175, 80, 0.2);
    padding: 10px 16px;
    border-radius: 6px;
    margin-bottom: 16px;
    font-size: 13px;
  }

  .published-banner code {
    background: rgba(0,0,0,0.2);
    padding: 2px 4px;
    border-radius: 4px;
  }

  .codegen-tab {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px 0;
  }

  .codegen-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .codegen-select {
    background: var(--color-crush-surface);
    border: 1px solid var(--color-crush-border);
    color: white;
    padding: 4px 8px;
    border-radius: 6px;
    font-size: 12px;
  }

  .codegen-block {
    background: rgba(0,0,0,0.2);
    border: 1px solid var(--color-crush-border);
    border-radius: 6px;
    padding: 12px;
    font-size: 12px;
    overflow-x: auto;
    white-space: pre-wrap;
    word-break: break-all;
    margin: 0;
  }

  .endpoint-inline-link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: rgba(59, 130, 246, 0.1);
    color: #3b82f6;
    border: 1px solid rgba(59, 130, 246, 0.2);
    border-radius: 4px;
    padding: 2px 6px;
    font-size: 12px;
    cursor: pointer;
    vertical-align: middle;
    margin: 0 4px;
  }

  .endpoint-inline-link:hover {
    background: rgba(59, 130, 246, 0.2);
  }

  .notebook-cell {
    border: 1px solid var(--color-crush-border);
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.01);
    margin: 12px 0;
    padding: 12px;
  }

  .notebook-cell-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--color-crush-border);
    padding-bottom: 8px;
    margin-bottom: 8px;
  }

  .cell-label {
    font-size: 12px;
    font-weight: 600;
    color: var(--color-crush-text-muted);
  }
</style>
