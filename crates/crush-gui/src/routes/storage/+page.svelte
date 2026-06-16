<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { getCurrentWebview } from '@tauri-apps/api/webview';
  import Icon from '$lib/components/Icon.svelte';
  import Drawer from '$lib/components/Drawer.svelte';
  import ProgressRing from '$lib/components/ProgressRing.svelte';
  import SegmentedControl from '$lib/components/SegmentedControl.svelte';
  import Skeleton from '$lib/components/Skeleton.svelte';
  import Breadcrumb from '$lib/components/Breadcrumb.svelte';
  import { confirmAction, promptInput } from '$lib/stores/confirm.svelte.ts';
  import * as api from '$lib/tauri';
  import type { S3Connection, BucketInfo, ObjectInfo, ObjectMetadata } from '$lib/tauri';

  // --- State variables (Svelte 5 Runes) ---
  let connections = $state<S3Connection[]>([]);
  let activeConn = $state<S3Connection | null>(null);
  let buckets = $state<BucketInfo[]>([]);
  let selectedBucket = $state<string | null>(null);
  let objects = $state<ObjectInfo[]>([]);
  let currentPrefix = $state(''); // e.g. 'images/'
  let loading = $state(false);
  
  // Phase O View States
  let viewMode = $state<'grid' | 'list'>('grid');
  let isGridView = $derived(viewMode === 'grid');
  let isDragging = $state(false);
  let uploadProgress = $state<Record<string, number>>({});
  // Lazy image-thumbnail cache: object key → presigned URL ('' = failed/in-flight).
  let thumbCache = $state<Record<string, string>>({});
  const IMG_RE = /\.(png|jpe?g|gif|webp|avif|bmp|svg)$/i;
  
  // Selection
  let selectedKeys = $state<string[]>([]);
  let searchPattern = $state('');

  // Modals
  let showAddConnModal = $state(false);
  let showCreateBucketModal = $state(false);
  let showBucketSettingsModal = $state(false);
  let showMetadataModal = $state(false);
  let showPreviewModal = $state(false);
  let showPresignModal = $state(false);

  // Form states
  // Add Connection
  let connName = $state('');
  let connEndpoint = $state('');
  let connRegion = $state('us-east-1');
  let connAccessKey = $state('');
  let connSecretKey = $state('');
  let connPathStyle = $state(true);
  
  // Create Bucket
  let newBucketName = $state('');

  // Bucket Settings
  let bucketPolicyJson = $state('');
  let bucketPublicStatus = $state(false);
  let savingPolicy = $state(false);

  // Metadata Editor
  let activeMetadataObj = $state<ObjectMetadata | null>(null);
  let metaContentType = $state('application/octet-stream');
  let metaPairs = $state<{ key: string; value: string }[]>([]);
  let savingMetadata = $state(false);

  // Preview Object
  let previewKey = $state('');
  let previewContent = $state('');
  let previewLoading = $state(false);
  let previewUrl = $state('');

  // Presign Form
  let presignKey = $state('');
  let presignMethod = $state('GET');
  let presignTtl = $state(3600); // 1 hour
  let generatedPresignedUrl = $state('');
  let generatingUrl = $state(false);

  // --- Computed Derived States ---
  // Folder/prefix tree browsing
  let currentItems = $derived.by(() => {
    const foldersSet = new Set<string>();
    const filesList: any[] = [];
    
    // Fuzzy search filter
    const filteredObjects = objects.filter(obj => 
      obj.key.toLowerCase().includes(searchPattern.toLowerCase())
    );

    for (const obj of filteredObjects) {
      if (!obj.key.startsWith(currentPrefix)) continue;
      if (obj.key === currentPrefix) continue; // skip current directory placeholder
      
      const relativePath = obj.key.substring(currentPrefix.length);
      const slashIdx = relativePath.indexOf('/');
      
      if (slashIdx !== -1) {
        const folderName = relativePath.substring(0, slashIdx);
        foldersSet.add(folderName);
      } else {
        filesList.push({
          ...obj,
          name: relativePath,
        });
      }
    }
    
    const foldersList = Array.from(foldersSet).map(name => ({
      name,
      key: currentPrefix + name + '/',
      isFolder: true,
      size: 0,
      last_modified: null,
    }));
    
    return [
      ...foldersList,
      ...filesList.map(f => ({ ...f, isFolder: false })),
    ];
  });

  // Breadcrumbs navigation (Phase O1)
  let breadcrumbItems = $derived.by(() => {
    let items = [{ name: selectedBucket || 'Home', prefix: '' }];
    if (!currentPrefix) return items;
    const parts = currentPrefix.split('/').filter(Boolean);
    let cumulative = '';
    for (const part of parts) {
      cumulative += part + '/';
      items.push({ name: part, prefix: cumulative });
    }
    return items;
  });

  // Total bucket size / object count
  let bucketStats = $derived.by(() => {
    let count = objects.length;
    let size = objects.reduce((acc, o) => acc + o.size, 0);
    return { count, size };
  });

  // Lazily fetch presigned thumbnail URLs for image objects in the current view.
  // Capped so a folder of thousands of images doesn't fan out into thousands of
  // presign calls; the rest fall back to the file-type icon.
  async function ensureThumbs(items: any[]) {
    if (!activeConn || !selectedBucket) return;
    let fetched = 0;
    for (const it of items) {
      if (fetched >= 60) break;
      if (it.isFolder || !IMG_RE.test(it.name)) continue;
      if (thumbCache[it.key] !== undefined) continue;
      thumbCache[it.key] = ''; // mark in-flight to avoid duplicate fetches
      fetched++;
      try {
        thumbCache[it.key] = await api.storageGetPresignedUrl(activeConn, selectedBucket, it.key, 'GET', 300);
      } catch {
        thumbCache[it.key] = '';
      }
    }
  }

  $effect(() => {
    if (isGridView && currentItems.length) ensureThumbs(currentItems);
  });

  // --- Actions & Helpers ---
  onMount(async () => {
    await loadConnections();

    // Real OS-level drag-and-drop (Tauri webview event carries actual paths).
    try {
      unlistenDrop = await getCurrentWebview().onDragDropEvent((event) => {
        const p = event.payload;
        if (p.type === 'enter' || p.type === 'over') {
          if (selectedBucket) isDragging = true;
        } else if (p.type === 'leave') {
          isDragging = false;
        } else if (p.type === 'drop') {
          isDragging = false;
          if (selectedBucket && p.paths?.length) uploadPaths(p.paths);
        }
      });
    } catch (e) {
      console.error('drag-drop registration failed', e);
    }
  });

  onDestroy(() => {
    if (unlistenDrop) unlistenDrop();
  });

  async function loadConnections() {
    loading = true;
    try {
      connections = await api.storageGetConnections();
      if (connections.length > 0) {
        activeConn = connections[0];
        await loadBuckets();
      }
    } catch (e) {
      alert(`Failed to load S3 connections: ${String(e)}`);
    } finally {
      loading = false;
    }
  }

  async function selectConnection(conn: S3Connection) {
    activeConn = conn;
    selectedBucket = null;
    objects = [];
    currentPrefix = '';
    selectedKeys = [];
    await loadBuckets();
  }

  async function loadBuckets() {
    if (!activeConn) return;
    loading = true;
    try {
      buckets = await api.storageListBuckets(activeConn);
    } catch (e) {
      alert(`Failed to list buckets: ${String(e)}`);
      buckets = [];
    } finally {
      loading = false;
    }
  }

  async function selectBucket(bucketName: string) {
    selectedBucket = bucketName;
    currentPrefix = '';
    selectedKeys = [];
    await loadObjects();
  }

  async function loadObjects() {
    if (!activeConn || !selectedBucket) return;
    loading = true;
    thumbCache = {};
    try {
      objects = await api.storageListObjects(activeConn, selectedBucket);
    } catch (e) {
      alert(`Failed to list objects: ${String(e)}`);
      objects = [];
    } finally {
      loading = false;
    }
  }

  async function createBucket() {
    if (!activeConn || !newBucketName.trim()) return;
    loading = true;
    try {
      await api.storageCreateBucket(activeConn, newBucketName.trim());
      await loadBuckets();
      showCreateBucketModal = false;
      newBucketName = '';
    } catch (e) {
      alert(`Failed to create bucket: ${String(e)}`);
    } finally {
      loading = false;
    }
  }

  async function deleteBucket(bucketName: string) {
    if (!activeConn) return;
    const force = await confirmAction({ title: 'Delete bucket', message: `Bucket "${bucketName}" may contain objects. Force delete the bucket and all its objects?`, confirmText: 'Delete bucket', danger: true });
    if (!force) return;
    loading = true;
    try {
      await api.storageDeleteBucket(activeConn, bucketName, true);
      if (selectedBucket === bucketName) {
        selectedBucket = null;
        objects = [];
      }
      await loadBuckets();
    } catch (e) {
      alert(`Failed to delete bucket: ${String(e)}`);
    } finally {
      loading = false;
    }
  }

  // Real batch upload progress (files completed / total). The single-shot S3
  // PUT has no byte-level progress, so we report honest per-file completion.
  let uploadBatch = $state<{ done: number; total: number; name: string; errors: number } | null>(null);

  // Upload a set of real local file paths into the current bucket/prefix.
  async function uploadPaths(paths: string[]) {
    if (!activeConn || !selectedBucket || paths.length === 0) return;
    uploadBatch = { done: 0, total: paths.length, name: '', errors: 0 };
    for (const path of paths) {
      const filename = path.split(/[/\\]/).pop() || 'file';
      uploadBatch = { ...uploadBatch, name: filename };
      const key = currentPrefix + filename;
      try {
        await api.storageUploadObject(activeConn, selectedBucket, key, path);
      } catch (e) {
        console.error('upload failed', key, e);
        uploadBatch = { ...uploadBatch, errors: uploadBatch.errors + 1 };
      }
      uploadBatch = { ...uploadBatch, done: uploadBatch.done + 1 };
    }
    const errs = uploadBatch.errors;
    uploadBatch = null;
    await loadObjects();
    if (errs > 0) alert(`${errs} file(s) failed to upload.`);
  }

  // Object Actions
  async function uploadFile() {
    if (!activeConn || !selectedBucket) return;
    try {
      const path = await api.storagePickUploadFile();
      if (!path) return;
      await uploadPaths([path]);
    } catch (e) {
      alert(`Upload failed: ${String(e)}`);
    }
  }

  // Phase O3 Drag and Drop — wired to Tauri's webview drag-drop events
  // (registered in onMount), which carry real filesystem paths. The browser
  // DragEvent path can't see real paths in a Tauri webview, so it's a no-op.
  let unlistenDrop: (() => void) | null = null;
  async function handleDrop(e: DragEvent) {
    e.preventDefault();
    isDragging = false;
  }

  async function downloadObject(item: ObjectInfo) {
    if (!activeConn || !selectedBucket) return;
    try {
      const filename = item.key.split('/').pop() || 'file';
      const savePath = await api.storagePickDownloadDestination(filename);
      if (!savePath) return;
      
      loading = true;
      await api.storageDownloadObject(activeConn, selectedBucket, item.key, savePath);
      alert(`Downloaded successfully to ${savePath}`);
    } catch (e) {
      alert(`Download failed: ${String(e)}`);
    } finally {
      loading = false;
    }
  }

  async function deleteSelectedObjects() {
    if (!activeConn || !selectedBucket || selectedKeys.length === 0) return;
    if (!await confirmAction({ title: 'Delete objects', message: `Delete ${selectedKeys.length} selected object(s)?`, confirmText: 'Delete', danger: true })) return;
    
    loading = true;
    try {
      await api.storageDeleteObjects(activeConn, selectedBucket, selectedKeys);
      selectedKeys = [];
      await loadObjects();
    } catch (e) {
      alert(`Delete failed: ${String(e)}`);
    } finally {
      loading = false;
    }
  }

  async function createFolder() {
    if (!activeConn || !selectedBucket) return;
    const folderName = await promptInput({ title: 'New folder', placeholder: 'images', confirmText: 'Create' });
    if (!folderName || !folderName.trim()) return;
    
    loading = true;
    try {
      // S3 folders are virtual, so we create a placeholder `.keep` file
      const key = currentPrefix + folderName.trim() + '/.keep';
      await api.storageUploadBytes(activeConn, selectedBucket, key, '', 'text/plain');
      await loadObjects();
    } catch (e) {
      alert(`Failed to create folder: ${String(e)}`);
    } finally {
      loading = false;
    }
  }

  // Connection form actions
  async function addConnection() {
    if (!connName.trim() || !connAccessKey.trim() || !connSecretKey.trim()) {
      alert('Name, Access Key, and Secret Key are required.');
      return;
    }
    const newConn: S3Connection = {
      name: connName.trim(),
      endpoint: connEndpoint.trim(),
      region: connRegion.trim(),
      access_key: connAccessKey.trim(),
      secret_key: connSecretKey.trim(),
      path_style: connPathStyle,
    };
    
    try {
      const updated = [...connections, newConn];
      await api.storageSaveConnections(updated);
      connections = updated;
      activeConn = newConn;
      showAddConnModal = false;
      
      // Reset form
      connName = '';
      connEndpoint = '';
      connRegion = 'us-east-1';
      connAccessKey = '';
      connSecretKey = '';
      connPathStyle = true;
      
      await loadBuckets();
    } catch (e) {
      alert(`Failed to save connection: ${String(e)}`);
    }
  }

  async function deleteActiveConnection() {
    if (!activeConn) return;
    if (activeConn.name === 'Local MinIO') {
      alert('Cannot delete default local MinIO connection.');
      return;
    }
    if (!await confirmAction({ title: 'Delete connection', message: `Delete S3 connection "${activeConn.name}"?`, confirmText: 'Delete', danger: true })) return;
    
    try {
      const updated = connections.filter(c => c.name !== activeConn!.name);
      await api.storageSaveConnections(updated);
      connections = updated;
      if (connections.length > 0) {
        activeConn = connections[0];
        await loadBuckets();
      } else {
        activeConn = null;
        buckets = [];
      }
    } catch (e) {
      alert(String(e));
    }
  }

  // Access Control policy helpers
  async function openBucketSettings(bucketName: string) {
    if (!activeConn) return;
    selectedBucket = bucketName;
    loading = true;
    try {
      bucketPolicyJson = await api.storageGetBucketPolicy(activeConn, bucketName);
      bucketPublicStatus = bucketPolicyJson.includes('PublicReadGetObject');
      showBucketSettingsModal = true;
    } catch (e) {
      alert(`Failed to load bucket settings: ${String(e)}`);
    } finally {
      loading = false;
    }
  }

  async function saveBucketSettings() {
    if (!activeConn || !selectedBucket) return;
    savingPolicy = true;
    try {
      // Toggle policy first
      await api.storageSetBucketPublic(activeConn, selectedBucket, bucketPublicStatus);
      // Save raw policy if set
      if (bucketPolicyJson.trim()) {
        await api.storageSetBucketPolicy(activeConn, selectedBucket, bucketPolicyJson);
      }
      showBucketSettingsModal = false;
      alert('Bucket settings saved.');
    } catch (e) {
      alert(`Failed to save policy: ${String(e)}`);
    } finally {
      savingPolicy = false;
    }
  }

  // Metadata editor helper
  async function openMetadata(key: string) {
    if (!activeConn || !selectedBucket) return;
    loading = true;
    try {
      activeMetadataObj = await api.storageGetObjectMetadata(activeConn, selectedBucket, key);
      metaContentType = activeMetadataObj.content_type || 'application/octet-stream';
      metaPairs = Object.entries(activeMetadataObj.metadata).map(([k, v]) => ({ key: k, value: v }));
      showMetadataModal = true;
    } catch (e) {
      alert(`Failed to fetch metadata: ${String(e)}`);
    } finally {
      loading = false;
    }
  }

  async function saveMetadata() {
    if (!activeConn || !selectedBucket || !activeMetadataObj) return;
    savingMetadata = true;
    try {
      const metadataMap: Record<string, string> = {};
      for (const pair of metaPairs) {
        if (pair.key.trim()) {
          metadataMap[pair.key.trim().toLowerCase()] = pair.value.trim();
        }
      }
      await api.storageSetObjectMetadata(activeConn, selectedBucket, activeMetadataObj.key, metaContentType, metadataMap);
      showMetadataModal = false;
      await loadObjects();
    } catch (e) {
      alert(`Failed to save metadata: ${String(e)}`);
    } finally {
      savingMetadata = false;
    }
  }

  // Presign Wizard
  function openPresign(key: string) {
    presignKey = key;
    presignMethod = 'GET';
    presignTtl = 3600;
    generatedPresignedUrl = '';
    showPresignModal = true;
  }

  async function generatePresignedUrl() {
    if (!activeConn || !selectedBucket) return;
    generatingUrl = true;
    try {
      generatedPresignedUrl = await api.storageGetPresignedUrl(activeConn, selectedBucket, presignKey, presignMethod, presignTtl);
    } catch (e) {
      alert(`Failed to generate URL: ${String(e)}`);
    } finally {
      generatingUrl = false;
    }
  }

  // Inline Previewer
  async function openPreview(item: ObjectInfo) {
    if (!activeConn || !selectedBucket) return;
    previewKey = item.key;
    previewLoading = true;
    previewContent = '';
    previewUrl = '';
    showPreviewModal = true;
    
    try {
      // Get temporary presigned URL for direct image/media loading
      previewUrl = await api.storageGetPresignedUrl(activeConn, selectedBucket, item.key, 'GET', 300);
      
      const ext = item.key.split('.').pop()?.toLowerCase();
      const isMedia = ['png', 'jpg', 'jpeg', 'gif', 'svg', 'mp4', 'mp3', 'pdf'].includes(ext || '');
      
      if (!isMedia) {
        // Fetch text preview
        previewContent = await api.storageReadObjectPreview(activeConn, selectedBucket, item.key);
      }
    } catch (e) {
      previewContent = `Failed to load preview: ${String(e)}`;
    } finally {
      previewLoading = false;
    }
  }

  // Sync / folder mirror helper
  let syncing = $state(false);
  async function syncLocalFolder() {
    if (!activeConn || !selectedBucket) return;
    try {
      const folderPath = await api.pickProjectDirectory();
      if (!folderPath) return;

      syncing = true;
      // Recursively mirror the local directory into the bucket under the current prefix.
      const res = await api.storageUploadDirectory(activeConn, selectedBucket, currentPrefix, folderPath);
      await loadObjects();
      const mb = (res.total_bytes / (1024 * 1024)).toFixed(1);
      let msg = `Synced ${res.uploaded}/${res.total_files} files (${mb} MB) into "${selectedBucket}".`;
      if (res.errors.length) msg += `\n\n${res.errors.length} failed:\n${res.errors.slice(0, 5).join('\n')}`;
      alert(msg);
    } catch (e) {
      alert(`Sync failed: ${String(e)}`);
    } finally {
      syncing = false;
    }
  }

  // Formatting utilities
  function formatBytes(bytes: number) {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }

  function formatDateTime(epochMs: number | null) {
    if (!epochMs) return '—';
    return new Date(epochMs).toLocaleString();
  }

  // Checkbox toggle
  function toggleSelection(key: string) {
    if (selectedKeys.includes(key)) {
      selectedKeys = selectedKeys.filter(k => k !== key);
    } else {
      selectedKeys = [...selectedKeys, key];
    }
  }
</script>

<div class="studio-container">
  <!-- Header -->
  <header class="studio-header">
    <div class="brand-section">
      <Icon name="box" size={18} class="text-primary-500" />
      <span class="studio-title">Local Storage Studio</span>
      {#if activeConn}
        <div class="active-badge">
          <span class="pulse-dot"></span>
          <span>{activeConn.name}</span>
        </div>
      {/if}
    </div>
    
    <div class="actions-section">
      {#if activeConn && activeConn.name !== 'Local MinIO'}
        <button class="disconnect-btn" onclick={deleteActiveConnection}>Delete Connection</button>
      {/if}
      <button class="switcher-btn" onclick={() => showAddConnModal = true}>
        <Icon name="sparkles" size={13} /> Add S3 Endpoint
      </button>
    </div>
  </header>

  <!-- Workspace -->
  <div class="studio-workspace">
    <!-- Sidebar: Connections + Buckets -->
    <aside class="sidebar-panel">
      <!-- Connection selection -->
      <div class="card-section">
        <label for="endpoint-select" class="sec-label">S3 Endpoint</label>
        <select 
          id="endpoint-select" 
          class="crush-input w-full mt-1" 
          value={activeConn?.name || ''} 
          onchange={(e) => {
            const target = e.currentTarget;
            const found = connections.find(c => c.name === target.value);
            if (found) selectConnection(found);
          }}
        >
          {#each connections as conn}
            <option value={conn.name}>{conn.name}</option>
          {/each}
        </select>
      </div>

      <!-- Bucket List -->
      <div class="card-section flex-1 flex flex-col mt-4">
        <div class="flex justify-between items-center mb-2">
          <span class="sec-label">Buckets</span>
          <button class="btn sm" onclick={() => showCreateBucketModal = true}>
            <Icon name="sparkles" size={12} /> New
          </button>
        </div>
        <div class="sidebar-list flex-1 overflow-y-auto mt-2">
          {#if buckets.length === 0}
            <p class="muted p-4 text-center">No buckets found</p>
          {:else}
            {#each buckets as b}
              <div class="sidebar-item" class:active={selectedBucket === b.name} role="button" tabindex="0" onclick={() => selectBucket(b.name)} onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') selectBucket(b.name); }}>
                <Icon name="folder" size={14} />
                <span class="bucket-name text-ellipsis">{b.name}</span>
                <div class="actions ml-auto">
                  <button class="ghost-btn sm" onclick={(e) => { e.stopPropagation(); openBucketSettings(b.name); }} title="Access Policy">
                    <Icon name="settings" size={11} />
                  </button>
                  <button class="ghost-btn sm text-red" onclick={(e) => { e.stopPropagation(); deleteBucket(b.name); }} title="Delete Bucket">
                    <Icon name="trash" size={11} />
                  </button>
                </div>
              </div>
            {/each}
          {/if}
        </div>
      </div>
    </aside>

    <!-- Main Content Area: Object Browser -->
    <main class="main-panel">
      {#if !selectedBucket}
        <div class="empty-workspace">
          <Icon name="box" size={48} class="muted mb-4" />
          <h3>No bucket selected</h3>
          <p class="muted">Select a bucket from the sidebar or create a new one to browse objects.</p>
        </div>
      {:else}
        <!-- Top bar: Path Breadcrumbs + Info -->
        <div class="workspace-header crush-card animate-slide-up">
          <div class="left-section">
            <div class="breadcrumbs">
              <Breadcrumb
                items={breadcrumbItems.map(c => ({ label: c.name, value: c.prefix }))}
                onnavigate={(prefix) => { currentPrefix = prefix; selectedKeys = []; }}
              />
            </div>
            {#if objects.length > 0}
              <div class="stats mt-1">
                <span>{bucketStats.count} objects</span>
                <span class="dot">•</span>
                <span>{formatBytes(bucketStats.size)} total</span>
              </div>
            {/if}
          </div>

          <div class="right-section flex gap-2">
            <SegmentedControl
              options={[{ value: 'grid', label: 'Grid' }, { value: 'list', label: 'List' }]}
              bind:selected={viewMode}
            />
            <input 
              type="text" 
              class="crush-input search-input" 
              placeholder="Search objects..." 
              bind:value={searchPattern}
            />
            <button class="btn" onclick={loadObjects} title="Refresh Objects">
              <Icon name="refresh" size={14} />
            </button>
            <button class="btn" onclick={createFolder} title="New Folder">
              <Icon name="folder" size={14} /> Folder
            </button>
            <button class="btn" onclick={syncLocalFolder} disabled={syncing} title="Mirror a local folder into this bucket">
              <Icon name="refresh" size={14} /> {syncing ? 'Syncing…' : 'Sync Folder'}
            </button>
            <button class="btn primary" onclick={uploadFile}>
              <Icon name="sparkles" size={14} /> Upload File
            </button>
          </div>
        </div>

        <!-- Multi-select action bar -->
        {#if selectedKeys.length > 0}
          <div class="multi-select-bar crush-card mt-3 animate-slide-up">
            <span>{selectedKeys.length} items selected</span>
            <button class="btn danger sm" onclick={deleteSelectedObjects}>
              <Icon name="trash" size={12} /> Delete Selected
            </button>
          </div>
        {/if}

        <!-- Phase O3 Drag and Drop Zone -->
        <!-- svelte-ignore a11y-no-static-element-interactions -->
        <div 
          class="object-list-card crush-card mt-4 animate-slide-up flex-1 flex flex-col relative transition-all duration-200"
          class:border-primary-500={isDragging}
          class:bg-primary-500={isDragging}
          class:bg-opacity-10={isDragging}
          ondragover={(e) => { e.preventDefault(); isDragging = true; }}
          ondragleave={() => isDragging = false}
          ondrop={handleDrop}
        >
          {#if isDragging}
            <div class="absolute inset-0 flex flex-col items-center justify-center z-10 pointer-events-none text-primary-400">
              <Icon name="sparkles" size={48} class="mb-4" />
              <h2 class="text-xl font-bold">Drop files to upload to {selectedBucket}</h2>
            </div>
          {/if}

          {#if uploadBatch}
            <div class="upload-banner crush-card">
              <ProgressRing value={Math.round((uploadBatch.done / uploadBatch.total) * 100)} size={36} strokeWidth={4} />
              <div class="upload-banner-text">
                <div class="upload-banner-title">Uploading {uploadBatch.done} / {uploadBatch.total}</div>
                <div class="upload-banner-sub">{uploadBatch.name}{uploadBatch.errors ? ` · ${uploadBatch.errors} failed` : ''}</div>
              </div>
            </div>
          {/if}

          <!-- Phase O2 Masonry Grid Toggle -->
          <div class="flex-1 overflow-y-auto p-4">
            {#if loading && objects.length === 0}
              <!-- Loading skeletons -->
              {#if isGridView}
                <div class="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-6 gap-4">
                  {#each Array(12) as _}
                    <div class="flex flex-col items-center p-4 border border-[var(--border)] rounded-lg gap-2">
                      <Skeleton width="48px" height="48px" />
                      <Skeleton width="80%" height="0.8rem" />
                      <Skeleton width="50%" height="0.7rem" />
                    </div>
                  {/each}
                </div>
              {:else}
                <div class="flex flex-col gap-2">
                  {#each Array(10) as _}
                    <Skeleton width="100%" height="2.2rem" />
                  {/each}
                </div>
              {/if}
            {:else if isGridView}
              <div class="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-6 gap-4">
                {#if currentPrefix}
                  <button class="flex flex-col items-center justify-center p-4 border border-[var(--border)] rounded-lg hover:bg-surface-hover hover:-translate-y-1 transition-all" onclick={() => {
                    const parts = currentPrefix.split('/').filter(Boolean);
                    parts.pop();
                    currentPrefix = parts.length > 0 ? parts.join('/') + '/' : '';
                    selectedKeys = [];
                  }}>
                    <Icon name="folder" size={32} class="text-primary-400 mb-2" />
                    <span class="text-sm font-medium">.. (Up)</span>
                  </button>
                {/if}
                
                {#each currentItems as item}
                  <button class="group flex flex-col items-center p-4 border rounded-lg transition-all relative outline-none"
                    class:border-primary-500={selectedKeys.includes(item.key)}
                    class:border-[var(--border)]={!selectedKeys.includes(item.key)}
                    class:hover:bg-surface-hover={true}
                    class:hover:-translate-y-1={true}
                    onclick={(e) => {
                      if (e.metaKey || e.ctrlKey) toggleSelection(item.key);
                      else if (item.isFolder) { currentPrefix = item.key; selectedKeys = []; }
                      else openPreview(item);
                    }}
                  >
                    <!-- Checkbox for multi-select in grid -->
                    {#if !item.isFolder}
                      <div class="absolute top-2 left-2 opacity-0 group-hover:opacity-100 transition-opacity" class:opacity-100={selectedKeys.includes(item.key)}>
                        <input type="checkbox" checked={selectedKeys.includes(item.key)} onclick={(e) => e.stopPropagation()} onchange={() => toggleSelection(item.key)} />
                      </div>
                    {/if}

                    {#if uploadProgress[item.key] !== undefined}
                      <div class="mb-2">
                        <ProgressRing value={uploadProgress[item.key]} size={32} strokeWidth={3} />
                      </div>
                    {:else if item.isFolder}
                      <Icon name="folder" size={32} class="text-primary-400 mb-2 group-hover:scale-110 transition-transform" />
                    {:else if IMG_RE.test(item.name) && thumbCache[item.key]}
                      <div class="thumb-wrap mb-2">
                        <img src={thumbCache[item.key]} alt={item.name} class="thumb-img" loading="lazy" />
                      </div>
                    {:else if IMG_RE.test(item.name)}
                      <div class="thumb-wrap mb-2"><Skeleton width="100%" height="100%" /></div>
                    {:else}
                      <Icon name="mail" size={32} class="text-text-muted mb-2 group-hover:scale-110 transition-transform" />
                    {/if}

                    <span class="text-sm font-medium w-full text-center truncate" title={item.name}>{item.name}</span>
                    <span class="text-xs text-text-muted mt-1">{item.isFolder ? 'Folder' : formatBytes(item.size)}</span>
                    
                    <!-- Quick actions on hover -->
                    {#if !item.isFolder}
                      <div class="absolute top-2 right-2 flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                        <div class="bg-surface border border-[var(--border)] rounded flex p-1 shadow-sm" onclick={(e) => e.stopPropagation()}>
                          <button class="p-1 hover:text-white" onclick={() => openMetadata(item.key)} title="Details"><Icon name="settings" size={12}/></button>
                        </div>
                      </div>
                    {/if}
                  </button>
                {/each}
              </div>
            {:else}
              <div class="ctable">
                <div class="crow chead obj-row">
                  <span class="checkbox-col"></span>
                  <span>Name</span>
                  <span>Size</span>
                  <span>Last Modified</span>
                  <span>Actions</span>
                </div>

                {#if currentPrefix}
                  <div class="crow obj-row folder-up" role="button" tabindex="0" onclick={() => {
                      const parts = currentPrefix.split('/').filter(Boolean);
                      parts.pop();
                      currentPrefix = parts.length > 0 ? parts.join('/') + '/' : '';
                      selectedKeys = [];
                    }}
                  >
                    <span class="checkbox-col"></span>
                    <span class="flex items-center gap-2">
                      <Icon name="folder" size={14} class="text-primary-400" />
                      <strong>.. (Up one level)</strong>
                    </span>
                    <span>—</span>
                    <span>—</span>
                    <span>—</span>
                  </div>
                {/if}

                {#if currentItems.length === 0}
                  <div class="crow text-center muted p-8"><span>No objects in this path</span></div>
                {:else}
                  {#each currentItems as item}
                    <div class="crow obj-row" class:selected={selectedKeys.includes(item.key)}>
                      <span class="checkbox-col">
                        {#if !item.isFolder}
                          <input type="checkbox" checked={selectedKeys.includes(item.key)} onchange={() => toggleSelection(item.key)} />
                        {/if}
                      </span>
                      
                      {#if item.isFolder}
                        <div class="flex items-center gap-2 cursor-pointer font-bold w-full" role="button" tabindex="0" onclick={() => { currentPrefix = item.key; selectedKeys = []; }}>
                          <Icon name="folder" size={14} class="text-primary-400" />
                          <span class="text-ellipsis">{item.name}</span>
                        </div>
                      {:else}
                        <div class="flex items-center gap-2 cursor-pointer w-full text-ellipsis" role="button" tabindex="0" onclick={() => openPreview(item)}>
                          {#if uploadProgress[item.key] !== undefined}
                            <ProgressRing value={uploadProgress[item.key]} size={16} strokeWidth={2} />
                          {:else}
                            <Icon name="mail" size={14} class="muted" />
                          {/if}
                          <span class="text-ellipsis">{item.name}</span>
                        </div>
                      {/if}

                      <span class="mono">{item.isFolder ? '—' : formatBytes(item.size)}</span>
                      <span class="mono dim">{item.isFolder ? '—' : formatDateTime(item.last_modified)}</span>
                      
                      <div class="actions">
                        {#if !item.isFolder}
                          <button class="ghost-btn sm" onclick={() => openMetadata(item.key)} title="View Metadata">Details</button>
                          <button class="ghost-btn sm" onclick={() => downloadObject(item)} title="Download file">Download</button>
                          <button class="ghost-btn sm text-red" onclick={() => { selectedKeys = [item.key]; deleteSelectedObjects(); }} title="Delete">
                            <Icon name="trash" size={12} />
                          </button>
                        {:else}
                          <span>—</span>
                        {/if}
                      </div>
                    </div>
                  {/each}
                {/if}
              </div>
            {/if}
          </div>
        </div>
      {/if}
    </main>
  </div>
</div>

<!-- Modal Dialogs -->

<!-- Add S3 Connection Endpoint Modal -->
{#if showAddConnModal}
  <div class="modal-backdrop" role="button" tabindex="-1" onclick={() => showAddConnModal = false} onkeydown={(e) => { if (e.key === 'Escape') showAddConnModal = false; }}>
    <div class="modal-card crush-card animate-slide-up" role="presentation" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
      <h3>Add S3 Connection</h3>
      <div class="modal-fields">
        <div class="field-row">
          <label for="conn-name-input">Connection Name</label>
          <input id="conn-name-input" type="text" class="crush-input" bind:value={connName} placeholder="Cloudflare R2" />
        </div>
        <div class="field-row">
          <label for="conn-endpoint-input">Endpoint URL (leave empty for AWS)</label>
          <input id="conn-endpoint-input" type="text" class="crush-input" bind:value={connEndpoint} placeholder="https://<acct>.r2.cloudflarestorage.com" />
        </div>
        <div class="field-row">
          <label for="conn-region-input">Region</label>
          <input id="conn-region-input" type="text" class="crush-input" bind:value={connRegion} placeholder="us-east-1" />
        </div>
        <div class="field-row">
          <label for="conn-access-input">Access Key</label>
          <input id="conn-access-input" type="text" class="crush-input" bind:value={connAccessKey} placeholder="access key id" />
        </div>
        <div class="field-row">
          <label for="conn-secret-input">Secret Key</label>
          <input id="conn-secret-input" type="password" class="crush-input" bind:value={connSecretKey} placeholder="secret access key" />
        </div>
        <div class="field-row checkbox-row">
          <label style="display: flex; align-items: center; gap: 8px; cursor: pointer;">
            <input type="checkbox" bind:checked={connPathStyle} /> Force Path-Style addressing (Needed for MinIO/Local)
          </label>
        </div>
      </div>
      <div class="modal-footer">
        <button class="btn" onclick={() => showAddConnModal = false}>Cancel</button>
        <button class="btn primary" onclick={addConnection}>Save Endpoint</button>
      </div>
    </div>
  </div>
{/if}

<!-- Create Bucket Modal -->
{#if showCreateBucketModal}
  <div class="modal-backdrop" role="button" tabindex="-1" onclick={() => showCreateBucketModal = false} onkeydown={(e) => { if (e.key === 'Escape') showCreateBucketModal = false; }}>
    <div class="modal-card crush-card animate-slide-up" role="presentation" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
      <h3>Create New Bucket</h3>
      <div class="modal-fields">
        <div class="field-row">
          <label for="bucket-name-input">Bucket Name</label>
          <input id="bucket-name-input" type="text" class="crush-input" bind:value={newBucketName} placeholder="assets" />
        </div>
      </div>
      <div class="modal-footer">
        <button class="btn" onclick={() => showCreateBucketModal = false}>Cancel</button>
        <button class="btn primary" onclick={createBucket}>Create</button>
      </div>
    </div>
  </div>
{/if}

<!-- Bucket Settings & Access Policy Modal -->
{#if showBucketSettingsModal}
  <div class="modal-backdrop" role="button" tabindex="-1" onclick={() => showBucketSettingsModal = false} onkeydown={(e) => { if (e.key === 'Escape') showBucketSettingsModal = false; }}>
    <div class="modal-card crush-card animate-slide-up" role="presentation" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
      <h3>Bucket Access Policy ({selectedBucket})</h3>
      <div class="modal-fields">
        <div class="field-row checkbox-row">
          <label style="display: flex; align-items: center; gap: 8px; cursor: pointer;">
            <input type="checkbox" bind:checked={bucketPublicStatus} /> Enable Public Read Access (Allows s3:GetObject publicly)
          </label>
        </div>
        <div class="field-row">
          <label for="raw-policy-text">Raw Bucket Policy JSON (Optional)</label>
          <textarea id="raw-policy-text" class="crush-input doc-editor" bind:value={bucketPolicyJson} placeholder="Raw policy JSON..."></textarea>
        </div>
      </div>
      <div class="modal-footer">
        <button class="btn" onclick={() => showBucketSettingsModal = false}>Cancel</button>
        <button class="btn primary" onclick={saveBucketSettings} disabled={savingPolicy}>
          {savingPolicy ? 'Saving...' : 'Save Settings'}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Phase O4 Object Metadata Details Panel (Drawer) -->
<Drawer bind:open={showMetadataModal} title="Object Details" side="right">
  {#if activeMetadataObj}
    <div class="flex flex-col gap-6">
      <div class="p-4 bg-surface-raised rounded-lg border border-[var(--border)]">
        <h4 class="text-sm font-semibold mb-2">Key</h4>
        <div class="font-mono text-xs text-text-muted break-all">{activeMetadataObj.key}</div>
      </div>

      <div class="field-row">
        <label for="meta-content-input" class="text-sm font-medium">Content Type</label>
        <input id="meta-content-input" type="text" class="crush-input mt-1" bind:value={metaContentType} />
      </div>
      
      <div>
        <div class="flex items-center justify-between mb-2">
          <span class="text-sm font-medium">Custom Metadata</span>
          <button class="btn sm" onclick={() => metaPairs = [...metaPairs, { key: '', value: '' }]}>
            <Icon name="plus" size={12} /> Add
          </button>
        </div>
        
        {#if metaPairs.length === 0}
          <div class="text-sm text-text-muted italic py-2">No custom tags</div>
        {/if}

        {#each metaPairs as pair, i}
          <div class="flex gap-2 mt-2 items-center">
            <input type="text" class="crush-input flex-1" placeholder="Key" bind:value={pair.key} />
            <input type="text" class="crush-input flex-1" placeholder="Value" bind:value={pair.value} />
            <button class="p-1 text-text-muted hover:text-red-400 transition-colors" onclick={() => metaPairs = metaPairs.filter((_, idx) => idx !== i)} title="Remove">
              <Icon name="x" size={14} />
            </button>
          </div>
        {/each}
      </div>

      <div class="flex gap-3 mt-4 pt-4 border-t border-[var(--border)]">
        <button class="btn flex-1" onclick={() => openPresign(activeMetadataObj!.key)}>
          <Icon name="link" size={14} /> Get Link
        </button>
        <button class="btn primary flex-1" onclick={saveMetadata} disabled={savingMetadata}>
          {savingMetadata ? 'Saving...' : 'Save Changes'}
        </button>
      </div>
    </div>
  {/if}
</Drawer>

<!-- Object Preview Modal -->
{#if showPreviewModal}
  <div class="modal-backdrop" role="button" tabindex="-1" onclick={() => showPreviewModal = false} onkeydown={(e) => { if (e.key === 'Escape') showPreviewModal = false; }}>
    <div class="modal-card crush-card animate-slide-up preview-modal-card" role="presentation" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
      <div class="modal-header flex justify-between items-center mb-4">
        <h3>Preview: {previewKey.split('/').pop()}</h3>
        <button class="ghost-btn sm" onclick={() => showPreviewModal = false}>
          <Icon name="x" size={14} />
        </button>
      </div>
      
      <div class="modal-fields preview-viewport">
        {#if previewLoading}
          <p class="muted p-8 text-center">Generating preview...</p>
        {:else}
          {#if previewKey.match(/\.(png|jpg|jpeg|gif|svg)$/i)}
            <img src={previewUrl} alt="Preview" class="preview-img" />
          {:else if previewKey.match(/\.(mp4)$/i)}
            <video src={previewUrl} controls class="preview-video"><track kind="captions" /></video>
          {:else if previewKey.match(/\.(mp3)$/i)}
            <audio src={previewUrl} controls class="preview-audio"></audio>
          {:else if previewContent}
            <pre class="preview-text">{previewContent}</pre>
          {:else}
            <div class="text-center p-8">
              <Icon name="box" size={48} class="muted mb-2" />
              <p>No inline preview available for this file type.</p>
              <button class="btn primary mt-4" onclick={() => { showPreviewModal = false; const it = objects.find(o => o.key === previewKey); if (it) downloadObject(it); }}>
                Download File
              </button>
            </div>
          {/if}
        {/if}
      </div>
    </div>
  </div>
{/if}

<!-- Presign URL Wizard Modal -->
{#if showPresignModal}
  <div class="modal-backdrop" role="button" tabindex="-1" onclick={() => showPresignModal = false} onkeydown={(e) => { if (e.key === 'Escape') showPresignModal = false; }}>
    <div class="modal-card crush-card animate-slide-up" role="presentation" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
      <h3>Generate Presigned URL</h3>
      <div class="modal-fields">
        <div class="field-row">
          <label for="presign-key-lbl">Object Key</label>
          <input id="presign-key-lbl" type="text" class="crush-input" value={presignKey} disabled />
        </div>
        <div class="field-row">
          <label for="presign-method-select">Action</label>
          <select id="presign-method-select" class="crush-input w-full" bind:value={presignMethod}>
            <option value="GET">GET (Read/Download)</option>
            <option value="PUT">PUT (Upload)</option>
          </select>
        </div>
        <div class="field-row">
          <label for="presign-ttl-select">Expiry (TTL)</label>
          <select id="presign-ttl-select" class="crush-input w-full" bind:value={presignTtl}>
            <option value={900}>15 Minutes</option>
            <option value={3600}>1 Hour</option>
            <option value={86400}>24 Hours</option>
            <option value={604800}>7 Days</option>
          </select>
        </div>
        
        {#if generatedPresignedUrl}
          <div class="mt-4">
            <label for="generated-url-text">Generated URL</label>
            <textarea id="generated-url-text" class="crush-input url-output" value={generatedPresignedUrl} readonly onclick={(e) => e.currentTarget.select()}></textarea>
            <p class="muted text-xs mt-1">Click above to select all, then copy to clipboard.</p>
          </div>
        {/if}
      </div>
      <div class="modal-footer">
        <button class="btn" onclick={() => showPresignModal = false}>Close</button>
        <button class="btn primary" onclick={generatePresignedUrl} disabled={generatingUrl}>
          {generatingUrl ? 'Generating...' : 'Generate URL'}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  /* Grid image thumbnails (Phase O2) */
  .thumb-wrap {
    width: 100%;
    aspect-ratio: 1 / 1;
    border-radius: var(--radius-md, 8px);
    overflow: hidden;
    background: var(--color-crush-surface);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .thumb-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    transition: transform 0.18s ease;
  }
  .group:hover .thumb-img { transform: scale(1.06); }

  /* Real upload progress banner (Phase O3) */
  .upload-banner {
    position: absolute;
    bottom: 16px;
    right: 16px;
    z-index: 20;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 16px 10px 10px;
    border-radius: 10px;
    box-shadow: var(--elevation-3, 0 8px 24px rgba(0,0,0,0.4));
  }
  .upload-banner-title { font-size: 13px; font-weight: 600; color: var(--color-crush-text); }
  .upload-banner-sub {
    font-size: 11px;
    color: var(--color-crush-text-muted);
    max-width: 220px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .studio-container {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--color-crush-black);
    color: var(--color-crush-text);
  }

  .studio-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 24px;
    border-bottom: 1px solid var(--color-crush-border);
    background: var(--color-crush-dark);
  }

  .brand-section {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .studio-title {
    font-size: 15px;
    font-weight: 600;
  }

  .active-badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--color-crush-text-muted);
    background: var(--color-crush-surface);
    padding: 3px 8px;
    border-radius: 6px;
    border: 1px solid var(--color-crush-border);
  }

  .pulse-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-crush-green);
    box-shadow: 0 0 0 0 rgba(16, 185, 129, 0.4);
    animation: pulse 1.6s infinite;
  }

  @keyframes pulse {
    0% { transform: scale(0.95); box-shadow: 0 0 0 0 rgba(16, 185, 129, 0.5); }
    70% { transform: scale(1); box-shadow: 0 0 0 5px rgba(16, 185, 129, 0); }
    100% { transform: scale(0.95); box-shadow: 0 0 0 0 rgba(16, 185, 129, 0); }
  }

  .actions-section {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .switcher-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: var(--color-crush-surface);
    border: 1px solid var(--color-crush-border);
    color: var(--color-crush-text);
    border-radius: 6px;
    padding: 6px 12px;
    font-size: 12px;
    cursor: pointer;
  }

  .disconnect-btn {
    background: none;
    border: none;
    color: var(--color-crush-text-muted);
    font-size: 12px;
    cursor: pointer;
    padding: 6px 10px;
  }

  .disconnect-btn:hover {
    color: var(--color-crush-red);
  }

  .studio-workspace {
    display: flex;
    flex: 1;
    height: calc(100vh - 48px);
    overflow: hidden;
  }

  .sidebar-panel {
    width: 260px;
    background: var(--color-crush-dark);
    border-right: 1px solid var(--color-crush-border);
    padding: 16px;
    display: flex;
    flex-direction: column;
  }

  .sec-label {
    font-size: 11px;
    text-transform: uppercase;
    color: var(--color-crush-text-muted);
    letter-spacing: 0.05em;
    font-weight: 600;
  }

  .sidebar-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .sidebar-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 12px;
    border-radius: 6px;
    background: none;
    border: 1px solid transparent;
    cursor: pointer;
    text-align: left;
    color: var(--color-crush-text);
    font-size: 13px;
  }

  .sidebar-item:hover {
    background: var(--color-crush-surface);
    border-color: var(--color-crush-border);
  }

  .sidebar-item.active {
    background: rgba(255,255,255,0.06);
    border-color: var(--color-crush-border);
    font-weight: 500;
  }

  .sidebar-item .actions {
    display: flex;
    gap: 4px;
    opacity: 0;
    transition: opacity 0.2s;
  }

  .sidebar-item:hover .actions {
    opacity: 1;
  }

  .main-panel {
    flex: 1;
    padding: 24px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .empty-workspace {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    color: var(--color-crush-text-muted);
  }

  .workspace-header {
    background: var(--color-crush-dark);
    padding: 16px 20px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-radius: 8px;
    border: 1px solid var(--color-crush-border);
  }

  .breadcrumbs {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 16px;
    font-weight: 600;
  }

  .stats {
    font-size: 12px;
    color: var(--color-crush-text-muted);
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .multi-select-bar {
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.2);
    padding: 12px 20px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-radius: 8px;
  }

  .object-list-card {
    border-radius: 8px;
    border: 1px solid var(--color-crush-border);
    background: var(--color-crush-dark);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .ctable {
    display: flex;
    flex-direction: column;
  }

  .crow {
    display: grid;
    align-items: center;
    padding: 10px 16px;
    border-bottom: 1px solid var(--color-crush-border);
    font-size: 12px;
  }

  .crow.chead {
    background: var(--color-crush-surface);
    font-weight: 600;
    color: var(--color-crush-text-muted);
    border-top: none;
  }

  .obj-row {
    grid-template-columns: 40px 1fr 100px 160px 240px;
  }

  .obj-row.selected {
    background: rgba(255,255,255,0.02);
  }

  .obj-row.folder-up:hover {
    background: rgba(255,255,255,0.04);
    cursor: pointer;
  }

  .checkbox-col {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .text-ellipsis {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }


  .crow .actions {
    display: flex;
    gap: 6px;
    justify-content: flex-end;
  }

  .preview-modal-card {
    max-width: 800px;
    width: 90%;
  }

  .preview-viewport {
    background: var(--color-crush-black);
    border: 1px solid var(--color-crush-border);
    border-radius: 6px;
    min-height: 200px;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: auto;
  }

  .preview-img {
    max-width: 100%;
    max-height: 500px;
    object-fit: contain;
  }

  .preview-video {
    max-width: 100%;
    max-height: 500px;
  }

  .preview-audio {
    width: 100%;
    padding: 16px;
  }

  .preview-text {
    font-family: var(--font-mono);
    font-size: 12px;
    padding: 16px;
    white-space: pre-wrap;
    width: 100%;
    margin: 0;
  }

  .url-output {
    font-family: var(--font-mono);
    font-size: 11px;
    height: 80px;
    width: 100%;
  }
</style>
