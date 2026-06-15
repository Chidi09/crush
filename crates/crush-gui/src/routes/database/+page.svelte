<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from '$lib/components/Icon.svelte';
  import TechIcon from '$lib/components/TechIcon.svelte';
  import * as api from '$lib/tauri';
  import type { DbStatus, BackupFile, QueryResult, RedisKeyInfo } from '$lib/tauri';

  // --- State variables (Svelte 5 Runes) ---
  let status = $state<DbStatus | null>(null);
  let backups = $state<BackupFile[]>([]);
  let loading = $state(true);
  let backingUp = $state(false);

  let nativeServices = $state<api.NativeServiceSummary[]>([]);
  let activeConnection = $state<{
    name: string;
    kind: 'postgres' | 'mysql' | 'redis' | 'mongodb' | 'minio';
    url: string;
    port?: number;
    password?: string;
    database?: string;
  } | null>(null);

  // Switcher state
  let switcherOpen = $state(false);
  let showCustomConnect = $state(false);

  // Custom connection input fields
  let customUrl = $state('');
  let customKind = $state<'postgres' | 'mysql' | 'redis' | 'mongodb'>('postgres');
  let connectionError = $state<string | null>(null);
  let connecting = $state(false);

  // Studio tabs
  // Tabs for SQL (postgres / mysql): 'data' | 'sql' | 'schema' | 'extensions' | 'performance' | 'security' | 'backups'
  // Tab for redis: 'redis-keys'
  // Tab for mongodb: 'mongo-colls'
  let activeTab = $state<'data' | 'sql' | 'schema' | 'extensions' | 'performance' | 'security' | 'backups' | 'redis-keys' | 'mongo-colls' | 'pgmq'>('data');

  // --- SQL data tab states ---
  let tables = $state<{ schema: string; name: string }[]>([]);
  let tableSearch = $state('');
  let selectedTable = $state<{ schema: string; name: string } | null>(null);
  let columns = $state<{ name: string; type: string; nullable: boolean }[]>([]);
  let rows = $state<any[][]>([]);
  let dataPage = $state(0);
  let dataLimit = $state(50);
  let dataTotalRows = $state(0);
  let filterText = $state('');
  let dataLoading = $state(false);
  let tableSort = $state<{ column: string; desc: boolean } | null>(null);

  // Cell edit state
  let editingCell = $state<{ rowIdx: number; colName: string; value: string } | null>(null);

  // Insert row modal state
  let showInsertModal = $state(false);
  let insertFormValues = $state<Record<string, string>>({});

  // --- SQL editor states ---
  let sqlQuery = $state('SELECT * FROM ');
  let sqlResults = $state<QueryResult | null>(null);
  let sqlError = $state<string | null>(null);
  let sqlHistory = $state<string[]>([]);
  let sqlLoading = $state(false);

  // --- Schema states ---
  let schemaTables = $state<any[]>([]);

  // --- Postgres Extensions states ---
  let extensions = $state<any[]>([]);

  // --- Postgres Performance states ---
  let explainQuery = $state('');
  let explainResult = $state<string | null>(null);
  let slowQueries = $state<any[]>([]);

  // --- Postgres Security states ---
  let pgRoles = $state<any[]>([]);
  let rlsPolicies = $state<any[]>([]);
  let rlsSelectedTable = $state<string | null>(null);

  // --- Redis Keys states ---
  let redisKeys = $state<RedisKeyInfo[]>([]);
  let redisPattern = $state('*');
  let selectedRedisKey = $state<RedisKeyInfo | null>(null);
  let redisKeyValue = $state('');
  let redisKeyTtl = $state(-1);
  let redisKeyType = $state('string');
  let redisNewKeyName = $state('');
  let redisNewKeyType = $state('string');
  let redisNewKeyValue = $state('');
  let showAddRedisKeyModal = $state(false);
  let redisLoading = $state(false);

  // --- MongoDB states ---
  let mongoDbs = $state<string[]>([]);
  let selectedMongoDb = $state('');
  let mongoColls = $state<string[]>([]);
  let selectedMongoColl = $state<string | null>(null);
  let mongoDocs = $state<any[]>([]);
  let mongoFilter = $state('');
  let mongoPage = $state(0);
  let mongoLimit = $state(50);
  let showInsertMongoDocModal = $state(false);
  let insertMongoDocValue = $state('{\n  \n}');
  let editingMongoDoc = $state<{ index: number; content: string } | null>(null);
  let mongoLoading = $state(false);

  // --- PGMQ states ---
  let pgmqEnabled = $state(false);
  let pgmqQueues = $state<{ name: string; createdAt: string }[]>([]);
  let selectedPgmqQueue = $state<string | null>(null);
  let pgmqMessages = $state<{ msgId: any; readCt: any; enqueuedAt: string; vt: string; message: string }[]>([]);
  let pgmqMetrics = $state<{
    queue_name: string;
    queue_length: number;
    newest_msg_age_sec: number;
    oldest_msg_age_sec: number;
    total_messages: number;
  } | null>(null);
  let showCreateQueueModal = $state(false);
  let pgmqNewQueueName = $state('');
  let pgmqNewQueueUnlogged = $state(false);
  let showSendPgmqMessageModal = $state(false);
  let pgmqSendMessageValue = $state('{\n  "hello": "world"\n}');
  let pgmqLoading = $state(false);

  // Focus action for inline cell editor
  function focusOnMount(node: HTMLInputElement) {
    node.focus();
  }

  // --- Computed Derived States ---
  let filteredTables = $derived(
    tables.filter(t => t.name.toLowerCase().includes(tableSearch.toLowerCase()))
  );

  onMount(async () => {
    loading = true;
    try {
      status = await api.dbStatus();
      backups = await api.dbBackups();
      
      // Load native services
      nativeServices = await api.listNativeServices();
      
      // Load history
      let hist = localStorage.getItem('crush_sql_history');
      if (hist) {
        sqlHistory = JSON.parse(hist);
      }
      
      // If Postgres is running natively, auto-connect to it!
      let pg = nativeServices.find(s => s.kind === 'postgres');
      if (pg) {
        connectToService(pg);
      }
    } catch (e) {
      console.error(e);
    } finally {
      loading = false;
    }
  });

  async function connectToService(svc: api.NativeServiceSummary) {
    connecting = true;
    connectionError = null;
    try {
      let url = svc.connection_string;
      if (!url) {
        if (svc.kind === 'postgres') {
          url = `postgresql://postgres:postgres@localhost:${svc.port}/postgres`;
        } else if (svc.kind === 'mysql') {
          url = `mysql://root:crush@localhost:${svc.port}/crush`;
        } else if (svc.kind === 'redis-compat') {
          url = `redis://localhost:${svc.port}`;
        } else if (svc.kind === 'mongodb') {
          url = `mongodb://localhost:${svc.port}`;
        }
      }
      
      activeConnection = {
        name: `Managed ${svc.kind} (${svc.port})`,
        kind: svc.kind === 'redis-compat' ? 'redis' : svc.kind as any,
        url: url || '',
        port: svc.port,
        password: svc.kind === 'postgres' ? 'postgres' : svc.kind === 'mysql' ? 'crush' : undefined,
        database: svc.kind === 'postgres' ? 'postgres' : svc.kind === 'mysql' ? 'crush' : undefined,
      };

      selectDefaultTab(activeConnection.kind);
      await initializeConnectionState();
    } catch (e) {
      connectionError = String(e);
    } finally {
      connecting = false;
      switcherOpen = false;
    }
  }

  async function connectToCustom() {
    connecting = true;
    connectionError = null;
    try {
      if (!customUrl.trim()) throw new Error('Connection URL is required');
      
      activeConnection = {
        name: `Custom ${customKind} Connection`,
        kind: customKind,
        url: customUrl,
      };
      
      selectDefaultTab(activeConnection.kind);
      await initializeConnectionState();
      showCustomConnect = false;
    } catch (e) {
      connectionError = String(e);
      activeConnection = null;
    } finally {
      connecting = false;
    }
  }

  function selectDefaultTab(kind: string) {
    if (kind === 'postgres' || kind === 'mysql') {
      activeTab = 'data';
    } else if (kind === 'redis') {
      activeTab = 'redis-keys';
    } else if (kind === 'mongodb') {
      activeTab = 'mongo-colls';
    }
  }

  async function initializeConnectionState() {
    if (!activeConnection) return;
    
    if (activeConnection.kind === 'postgres' || activeConnection.kind === 'mysql') {
      await loadTables();
      if (tables.length > 0) {
        await selectTable(tables[0]);
      } else {
        selectedTable = null;
        columns = [];
        rows = [];
      }
      if (activeConnection.kind === 'postgres') {
        await loadExtensions();
        await loadRolesAndSecurity();
        await loadPerformanceStats();
        await checkPgmq();
      }
      await loadSchemaStats();
    } else if (activeConnection.kind === 'redis') {
      await loadRedisKeys();
    } else if (activeConnection.kind === 'mongodb') {
      await loadMongoDbs();
    }
  }

  // SQL Engines Helpers
  async function loadTables() {
    if (!activeConnection) return;
    let q = '';
    if (activeConnection.kind === 'postgres') {
      q = `SELECT table_schema as schema, table_name as name 
           FROM information_schema.tables 
           WHERE table_schema NOT IN ('pg_catalog', 'information_schema') 
           ORDER BY table_name;`;
    } else {
      q = `SELECT table_schema as schema, table_name as name 
           FROM information_schema.tables 
           WHERE table_schema NOT IN ('mysql', 'information_schema', 'performance_schema', 'sys') 
           ORDER BY table_name;`;
    }
    
    try {
      let res = await api.dbRunQuery(activeConnection.kind, activeConnection.url, q);
      if (res.error) {
        console.error(res.error);
        tables = [];
      } else {
        tables = res.rows.map(row => ({ schema: row[0], name: row[1] }));
      }
    } catch (e) {
      console.error(e);
      tables = [];
    }
  }

  async function selectTable(table: { schema: string; name: string }) {
    selectedTable = table;
    dataPage = 0;
    tableSort = null;
    filterText = '';
    await loadTableData(table);
  }

  function quoteIdent(ident: string, engine: string): string {
    if (engine === 'postgres') {
      return `"${ident.replace(/"/g, '""')}"`;
    } else {
      return `\`${ident.replace(/`/g, '``')}\``;
    }
  }

  function escapeSqlVal(val: any): string {
    if (val === null || val === undefined) return '';
    return String(val).replace(/'/g, "''");
  }

  async function loadTableData(table: { schema: string; name: string }) {
    if (!activeConnection) return;
    dataLoading = true;
    try {
      let colQ = `SELECT column_name, data_type, is_nullable 
                  FROM information_schema.columns 
                  WHERE table_schema = '${table.schema}' AND table_name = '${table.name}' 
                  ORDER BY ordinal_position;`;
      
      let colRes = await api.dbRunQuery(activeConnection.kind, activeConnection.url, colQ);
      if (colRes.error) throw new Error(colRes.error);
      
      columns = colRes.rows.map(row => ({
        name: row[0],
        type: row[1],
        nullable: row[2] === 'YES' || row[2] === 'yes'
      }));

      // Initialize insert form values
      let newForm: Record<string, string> = {};
      columns.forEach(c => {
        newForm[c.name] = '';
      });
      insertFormValues = newForm;

      // 2. Load Total Count
      let countQ = `SELECT count(*) FROM ${quoteIdent(table.schema, activeConnection.kind)}.${quoteIdent(table.name, activeConnection.kind)}`;
      if (filterText.trim()) {
        let conditions = columns.map(c => `${quoteIdent(c.name, activeConnection!.kind)}::text ILIKE '%${escapeSqlVal(filterText)}%'`).join(' OR ');
        if (activeConnection.kind === 'mysql') {
          conditions = columns.map(c => `${quoteIdent(c.name, activeConnection!.kind)} LIKE '%${escapeSqlVal(filterText)}%'`).join(' OR ');
        }
        countQ += ` WHERE ${conditions}`;
      }
      
      let countRes = await api.dbRunQuery(activeConnection.kind, activeConnection.url, countQ);
      if (!countRes.error && countRes.rows.length > 0) {
        dataTotalRows = parseInt(countRes.rows[0][0]);
      } else {
        dataTotalRows = 0;
      }

      // 3. Load Rows
      let dataQ = `SELECT * FROM ${quoteIdent(table.schema, activeConnection.kind)}.${quoteIdent(table.name, activeConnection.kind)}`;
      if (filterText.trim()) {
        let conditions = columns.map(c => `${quoteIdent(c.name, activeConnection!.kind)}::text ILIKE '%${escapeSqlVal(filterText)}%'`).join(' OR ');
        if (activeConnection.kind === 'mysql') {
          conditions = columns.map(c => `${quoteIdent(c.name, activeConnection!.kind)} LIKE '%${escapeSqlVal(filterText)}%'`).join(' OR ');
        }
        dataQ += ` WHERE ${conditions}`;
      }
      if (tableSort) {
        dataQ += ` ORDER BY ${quoteIdent(tableSort.column, activeConnection.kind)} ${tableSort.desc ? 'DESC' : 'ASC'}`;
      }
      dataQ += ` LIMIT ${dataLimit} OFFSET ${dataPage * dataLimit}`;

      let dataRes = await api.dbRunQuery(activeConnection.kind, activeConnection.url, dataQ);
      if (dataRes.error) throw new Error(dataRes.error);
      
      rows = dataRes.rows;
    } catch (e) {
      alert(`Failed to load table data: ${String(e)}`);
      rows = [];
    } finally {
      dataLoading = false;
    }
  }

  async function getPrimaryKeyCols(): Promise<string[]> {
    if (!activeConnection || !selectedTable) return [];
    let q = `SELECT kcu.column_name
             FROM information_schema.table_constraints tc
             JOIN information_schema.key_column_usage kcu
               ON tc.constraint_name = kcu.constraint_name
               AND tc.table_schema = kcu.table_schema
             WHERE tc.constraint_type = 'PRIMARY KEY'
               AND tc.table_name = '${selectedTable.name}'
               AND tc.table_schema = '${selectedTable.schema}';`;
    try {
      let res = await api.dbRunQuery(activeConnection.kind, activeConnection.url, q);
      if (res.error) return [];
      return res.rows.map(r => r[0]);
    } catch (e) {
      return [];
    }
  }

  async function saveCellEdit(rowIdx: number, colName: string, newValue: string, row: any[]) {
    if (!activeConnection || !selectedTable) return;
    
    try {
      let pkCols = await getPrimaryKeyCols();
      let whereClause = '';
      if (pkCols.length > 0) {
        whereClause = pkCols.map(c => {
          let colIdx = columns.findIndex(col => col.name === c);
          let val = row[colIdx];
          return `${quoteIdent(c, activeConnection!.kind)} = '${escapeSqlVal(val)}'`;
        }).join(' AND ');
      } else {
        whereClause = columns.map((col, idx) => {
          let val = row[idx];
          if (val === null) {
            return `${quoteIdent(col.name, activeConnection!.kind)} IS NULL`;
          } else {
            return `${quoteIdent(col.name, activeConnection!.kind)} = '${escapeSqlVal(val)}'`;
          }
        }).join(' AND ');
      }
      
      let q = `UPDATE ${quoteIdent(selectedTable.schema, activeConnection.kind)}.${quoteIdent(selectedTable.name, activeConnection.kind)} 
               SET ${quoteIdent(colName, activeConnection.kind)} = '${escapeSqlVal(newValue)}' 
               WHERE ${whereClause}`;
               
      let res = await api.dbRunQuery(activeConnection.kind, activeConnection.url, q);
      if (res.error) {
        alert(`Failed to save: ${res.error}`);
      } else {
        await loadTableData(selectedTable);
      }
    } catch (e) {
      alert(`Save failed: ${String(e)}`);
    } finally {
      editingCell = null;
    }
  }

  async function deleteRow(row: any[]) {
    if (!activeConnection || !selectedTable) return;
    if (!confirm('Are you sure you want to delete this row?')) return;
    
    try {
      let pkCols = await getPrimaryKeyCols();
      let whereClause = '';
      if (pkCols.length > 0) {
        whereClause = pkCols.map(c => {
          let colIdx = columns.findIndex(col => col.name === c);
          let val = row[colIdx];
          return `${quoteIdent(c, activeConnection!.kind)} = '${escapeSqlVal(val)}'`;
        }).join(' AND ');
      } else {
        whereClause = columns.map((col, idx) => {
          let val = row[idx];
          if (val === null) {
            return `${quoteIdent(col.name, activeConnection!.kind)} IS NULL`;
          } else {
            return `${quoteIdent(col.name, activeConnection!.kind)} = '${escapeSqlVal(val)}'`;
          }
        }).join(' AND ');
      }
      
      let q = `DELETE FROM ${quoteIdent(selectedTable.schema, activeConnection.kind)}.${quoteIdent(selectedTable.name, activeConnection.kind)} 
               WHERE ${whereClause}`;
               
      let res = await api.dbRunQuery(activeConnection.kind, activeConnection.url, q);
      if (res.error) {
        alert(`Failed to delete: ${res.error}`);
      } else {
        await loadTableData(selectedTable);
      }
    } catch (e) {
      alert(`Delete failed: ${String(e)}`);
    }
  }

  async function insertRow() {
    if (!activeConnection || !selectedTable) return;
    
    let cols: string[] = [];
    let vals: string[] = [];
    
    Object.entries(insertFormValues).forEach(([k, v]) => {
      if (v.trim() !== '') {
        cols.push(quoteIdent(k, activeConnection!.kind));
        vals.push(`'${escapeSqlVal(v)}'`);
      }
    });
    
    if (cols.length === 0) {
      alert('Please fill in at least one column');
      return;
    }
    
    let q = `INSERT INTO ${quoteIdent(selectedTable.schema, activeConnection.kind)}.${quoteIdent(selectedTable.name, activeConnection.kind)} 
             (${cols.join(', ')}) VALUES (${vals.join(', ')})`;
             
    try {
      let res = await api.dbRunQuery(activeConnection.kind, activeConnection.url, q);
      if (res.error) {
        alert(`Insert failed: ${res.error}`);
      } else {
        showInsertModal = false;
        await loadTableData(selectedTable);
      }
    } catch (e) {
      alert(`Insert failed: ${String(e)}`);
    }
  }

  // SQL Editor helpers
  async function runSQL() {
    if (!activeConnection) return;
    sqlLoading = true;
    sqlError = null;
    sqlResults = null;
    try {
      let isDestructive = /drop|truncate|alter|delete|update/i.test(sqlQuery) && 
        !(/where/i.test(sqlQuery));
      if (isDestructive) {
        if (!confirm(`Warning: You are running a potentially destructive query without a WHERE clause:\n\n${sqlQuery}\n\nDo you want to continue?`)) {
          sqlLoading = false;
          return;
        }
      }

      let res = await api.dbRunQuery(activeConnection.kind, activeConnection.url, sqlQuery);
      if (res.error) {
        sqlError = res.error;
      } else {
        sqlResults = res;
        if (!sqlHistory.includes(sqlQuery)) {
          sqlHistory = [sqlQuery, ...sqlHistory.slice(0, 19)];
          localStorage.setItem('crush_sql_history', JSON.stringify(sqlHistory));
        }
      }
    } catch (e) {
      sqlError = String(e);
    } finally {
      sqlLoading = false;
    }
  }

  // Schema Tab
  async function loadSchemaStats() {
    if (!activeConnection) return;
    let q = '';
    if (activeConnection.kind === 'postgres') {
      q = `SELECT 
             relname AS table_name,
             pg_size_pretty(pg_total_relation_size(c.oid)) AS total_size,
             pg_size_pretty(pg_relation_size(c.oid)) AS table_size,
             pg_size_pretty(pg_indexes_size(c.oid)) AS index_size,
             n_live_tup AS live_rows
           FROM pg_class c
           LEFT JOIN pg_namespace n ON n.oid = c.relnamespace
           LEFT JOIN pg_stat_user_tables s ON s.relname = c.relname
           WHERE c.relkind = 'r' AND n.nspname = 'public'
           ORDER BY pg_total_relation_size(c.oid) DESC;`;
    } else {
      q = `SELECT table_name, 
                  data_length as table_size, 
                  index_length as index_size, 
                  table_rows as live_rows 
           FROM information_schema.tables 
           WHERE table_schema = DATABASE()
           ORDER BY (data_length + index_length) DESC;`;
    }
    
    try {
      let res = await api.dbRunQuery(activeConnection.kind, activeConnection.url, q);
      if (!res.error) {
        schemaTables = res.rows;
      }
    } catch (e) {
      console.error(e);
    }
  }

  // Extensions Tab (Postgres)
  async function loadExtensions() {
    if (!activeConnection || activeConnection.kind !== 'postgres') return;
    let q = `SELECT name, default_version as version, comment as description, 
               EXISTS(SELECT 1 FROM pg_extension WHERE extname = name) as installed
             FROM pg_available_extensions
             ORDER BY installed DESC, name;`;
    try {
      let res = await api.dbRunQuery('postgres', activeConnection.url, q);
      if (!res.error) {
        extensions = res.rows.map(row => ({
          name: row[0],
          version: row[1],
          description: row[2],
          installed: row[3] === 'true' || row[3] === true
        }));
      }
    } catch (e) {
      console.error(e);
    }
  }

  async function toggleExtension(ext: any) {
    if (!activeConnection) return;
    let q = ext.installed 
      ? `DROP EXTENSION IF EXISTS "${ext.name}"` 
      : `CREATE EXTENSION IF NOT EXISTS "${ext.name}"`;
    try {
      let res = await api.dbRunQuery('postgres', activeConnection.url, q);
      if (res.error) {
        alert(`Failed to modify extension: ${res.error}`);
      } else {
        await loadExtensions();
        await checkPgmq();
      }
    } catch (e) {
      alert(String(e));
    }
  }

  // Performance Tab
  async function loadPerformanceStats() {
    if (!activeConnection || activeConnection.kind !== 'postgres') return;
    
    let extCheck = `SELECT 1 FROM pg_extension WHERE extname = 'pg_stat_statements';`;
    let extRes = await api.dbRunQuery('postgres', activeConnection.url, extCheck);
    if (extRes.rows.length > 0) {
      let q = `SELECT query, calls, total_exec_time, mean_exec_time, rows 
               FROM pg_stat_statements 
               ORDER BY total_exec_time DESC 
               LIMIT 10;`;
      let statRes = await api.dbRunQuery('postgres', activeConnection.url, q);
      if (!statRes.error) {
        slowQueries = statRes.rows;
      }
    } else {
      slowQueries = [];
    }
  }

  async function explain() {
    if (!activeConnection) return;
    explainResult = null;
    let q = `EXPLAIN ${explainQuery}`;
    try {
      let res = await api.dbRunQuery(activeConnection.kind, activeConnection.url, q);
      if (res.error) {
        alert(res.error);
      } else {
        explainResult = res.rows.map(row => row[0]).join('\n');
      }
    } catch (e) {
      alert(String(e));
    }
  }

  // Security Tab
  async function loadRolesAndSecurity() {
    if (!activeConnection || activeConnection.kind !== 'postgres') return;
    
    let rolesQ = `SELECT rolname, rolsuper, rolinherit, rolcreaterole, rolcreatedb, rolcanlogin FROM pg_roles;`;
    let policiesQ = `SELECT schemaname, tablename, policyname, cmd, roles FROM pg_policies;`;
    
    try {
      let rolesRes = await api.dbRunQuery('postgres', activeConnection.url, rolesQ);
      if (!rolesRes.error) pgRoles = rolesRes.rows;
      
      let policiesRes = await api.dbRunQuery('postgres', activeConnection.url, policiesQ);
      if (!policiesRes.error) rlsPolicies = policiesRes.rows;
    } catch (e) {
      console.error(e);
    }
  }

  async function toggleRLS(table: { schema: string; name: string }, enable: boolean) {
    if (!activeConnection) return;
    let q = `ALTER TABLE "${table.schema}"."${table.name}" ${enable ? 'ENABLE' : 'DISABLE'} ROW LEVEL SECURITY;`;
    try {
      let res = await api.dbRunQuery('postgres', activeConnection.url, q);
      if (res.error) {
        alert(res.error);
      } else {
        alert(`Row Level Security ${enable ? 'enabled' : 'disabled'} successfully.`);
        await loadRolesAndSecurity();
      }
    } catch (e) {
      alert(String(e));
    }
  }

  // Redis Key Browser helpers
  async function loadRedisKeys() {
    if (!activeConnection || activeConnection.kind !== 'redis') return;
    redisLoading = true;
    try {
      redisKeys = await api.redisListKeys(activeConnection.port || 6379, activeConnection.password, redisPattern);
      selectedRedisKey = null;
      redisKeyValue = '';
    } catch (e) {
      alert(String(e));
    } finally {
      redisLoading = false;
    }
  }

  async function selectRedisKey(key: RedisKeyInfo) {
    if (!activeConnection) return;
    selectedRedisKey = key;
    try {
      redisKeyValue = await api.redisGetVal(activeConnection.port || 6379, activeConnection.password, key.key);
      redisKeyTtl = key.ttl;
      redisKeyType = key.kind;
    } catch (e) {
      redisKeyValue = `<Failed to read key: ${String(e)}>`;
    }
  }

  async function saveRedisKey() {
    if (!activeConnection || !selectedRedisKey) return;
    try {
      await api.redisSetVal(activeConnection.port || 6379, activeConnection.password, selectedRedisKey.key, redisKeyValue, redisKeyTtl > 0 ? redisKeyTtl : undefined);
      alert('Key saved successfully.');
      await loadRedisKeys();
    } catch (e) {
      alert(String(e));
    }
  }

  async function deleteRedisKey(keyName: string) {
    if (!activeConnection) return;
    if (!confirm(`Are you sure you want to delete key "${keyName}"?`)) return;
    try {
      await api.redisDelKey(activeConnection.port || 6379, activeConnection.password, keyName);
      await loadRedisKeys();
    } catch (e) {
      alert(String(e));
    }
  }

  async function addRedisKey() {
    if (!activeConnection) return;
    if (!redisNewKeyName.trim()) {
      alert('Key name is required');
      return;
    }
    try {
      await api.redisSetVal(activeConnection.port || 6379, activeConnection.password, redisNewKeyName, redisNewKeyValue);
      showAddRedisKeyModal = false;
      redisNewKeyName = '';
      redisNewKeyValue = '';
      await loadRedisKeys();
    } catch (e) {
      alert(String(e));
    }
  }

  // MongoDB helpers
  async function loadMongoDbs() {
    if (!activeConnection || activeConnection.kind !== 'mongodb') return;
    try {
      mongoDbs = await api.mongoListDatabases(activeConnection.port || 27017);
      if (mongoDbs.length > 0) {
        selectedMongoDb = mongoDbs.includes('admin') ? 'admin' : mongoDbs[0];
        await loadMongoCollections();
      }
    } catch (e) {
      alert(String(e));
    }
  }

  async function loadMongoCollections() {
    if (!activeConnection || !selectedMongoDb) return;
    try {
      mongoColls = await api.mongoListCollections(activeConnection.port || 27017, selectedMongoDb);
      if (mongoColls.length > 0) {
        await selectMongoCollection(mongoColls[0]);
      } else {
        selectedMongoColl = null;
        mongoDocs = [];
      }
    } catch (e) {
      alert(String(e));
    }
  }

  async function selectMongoCollection(coll: string) {
    selectedMongoColl = coll;
    mongoPage = 0;
    mongoFilter = '';
    await loadMongoDocs();
  }

  async function loadMongoDocs() {
    if (!activeConnection || !selectedMongoDb || !selectedMongoColl) return;
    mongoLoading = true;
    try {
      let rawDocs = await api.mongoFindDocs(
        activeConnection.port || 27017,
        selectedMongoDb,
        selectedMongoColl,
        mongoFilter || undefined,
        mongoLimit,
        mongoPage * mongoLimit
      );
      mongoDocs = rawDocs.map(d => JSON.parse(d));
    } catch (e) {
      alert(`Mongo query failed: ${String(e)}`);
      mongoDocs = [];
    } finally {
      mongoLoading = false;
    }
  }

  async function saveMongoDoc(index: number) {
    if (!activeConnection || !selectedMongoDb || !selectedMongoColl || !editingMongoDoc) return;
    
    try {
      let oldDoc = mongoDocs[index];
      let newDoc = JSON.parse(editingMongoDoc.content);
      
      if (!oldDoc._id) {
        throw new Error('Cannot update document without an _id field');
      }
      
      let filter = JSON.stringify({ _id: oldDoc._id });
      let update = JSON.stringify({ $set: newDoc });
      
      let count = await api.mongoUpdateDoc(
        activeConnection.port || 27017,
        selectedMongoDb,
        selectedMongoColl,
        filter,
        update
      );
      
      if (count > 0) {
        alert('Document updated.');
        editingMongoDoc = null;
        await loadMongoDocs();
      } else {
        alert('Document not updated (no modifications).');
      }
    } catch (e) {
      alert(`Save failed: ${String(e)}`);
    }
  }

  async function deleteMongoDoc(doc: any) {
    if (!activeConnection || !selectedMongoDb || !selectedMongoColl) return;
    if (!confirm('Delete this document?')) return;
    
    try {
      if (!doc._id) {
        throw new Error('Cannot delete document without an _id field');
      }
      let filter = JSON.stringify({ _id: doc._id });
      let count = await api.mongoDeleteDoc(
        activeConnection.port || 27017,
        selectedMongoDb,
        selectedMongoColl,
        filter
      );
      if (count > 0) {
        await loadMongoDocs();
      }
    } catch (e) {
      alert(String(e));
    }
  }

  async function insertMongoDoc() {
    if (!activeConnection || !selectedMongoDb || !selectedMongoColl) return;
    try {
      JSON.parse(insertMongoDocValue);
      await api.mongoInsertDoc(
        activeConnection.port || 27017,
        selectedMongoDb,
        selectedMongoColl,
        insertMongoDocValue
      );
      showInsertMongoDocModal = false;
      insertMongoDocValue = '{\n  \n}';
      await loadMongoDocs();
    } catch (e) {
      alert(`Invalid JSON or insert error: ${String(e)}`);
    }
  }

  // --- PGMQ Queues helpers ---
  async function checkPgmq() {
    if (!activeConnection || activeConnection.kind !== 'postgres') return;
    try {
      let res = await api.dbRunQuery('postgres', activeConnection.url, "SELECT extname FROM pg_extension WHERE extname = 'pgmq';");
      pgmqEnabled = !res.error && res.rows && res.rows.length > 0;
      if (pgmqEnabled) {
        await loadPgmqQueues();
      }
    } catch (e) {
      console.error(e);
      pgmqEnabled = false;
    }
  }

  async function enablePgmq() {
    if (!activeConnection || activeConnection.kind !== 'postgres') return;
    pgmqLoading = true;
    try {
      let res = await api.dbRunQuery('postgres', activeConnection.url, "CREATE EXTENSION IF NOT EXISTS pgmq;");
      if (res.error) {
        alert(`Failed to enable PGMQ extension: ${res.error}`);
      } else {
        await checkPgmq();
      }
    } catch (e) {
      alert(`Error enabling PGMQ extension: ${String(e)}`);
    } finally {
      pgmqLoading = false;
    }
  }

  async function loadPgmqQueues() {
    if (!activeConnection || activeConnection.kind !== 'postgres') return;
    pgmqLoading = true;
    try {
      let res = await api.dbRunQuery('postgres', activeConnection.url, "SELECT queue_name, created_at::text FROM pgmq.list_queues();");
      if (res.error) {
        console.error(res.error);
        pgmqQueues = [];
      } else {
        pgmqQueues = res.rows.map(row => ({
          name: row[0],
          createdAt: row[1]
        }));
      }
    } catch (e) {
      console.error(e);
      pgmqQueues = [];
    } finally {
      pgmqLoading = false;
    }
  }

  async function selectPgmqQueue(name: string) {
    selectedPgmqQueue = name;
    await loadPgmqQueueData();
  }

  async function loadPgmqQueueData() {
    if (!activeConnection || activeConnection.kind !== 'postgres' || !selectedPgmqQueue) return;
    pgmqLoading = true;
    try {
      // Load metrics
      let metQ = `SELECT queue_name, queue_length, newest_msg_age_sec, oldest_msg_age_sec, total_messages FROM pgmq.metrics('${escapeSqlVal(selectedPgmqQueue)}');`;
      let metRes = await api.dbRunQuery('postgres', activeConnection.url, metQ);
      if (!metRes.error && metRes.rows && metRes.rows.length > 0) {
        let r = metRes.rows[0];
        pgmqMetrics = {
          queue_name: r[0],
          queue_length: parseInt(r[1]) || 0,
          newest_msg_age_sec: parseInt(r[2]) || 0,
          oldest_msg_age_sec: parseInt(r[3]) || 0,
          total_messages: parseInt(r[4]) || 0
        };
      } else {
        pgmqMetrics = null;
      }

      // Load messages from pgmq.q_<queue> table directly
      // Columns in pgmq.q_<queue>: msg_id, read_ct, enqueued_at, vt, message
      let msgQ = `SELECT msg_id, read_ct, enqueued_at::text, vt::text, message::text FROM pgmq.q_${selectedPgmqQueue} ORDER BY msg_id DESC LIMIT 100;`;
      let msgRes = await api.dbRunQuery('postgres', activeConnection.url, msgQ);
      if (!msgRes.error) {
        pgmqMessages = msgRes.rows.map(row => ({
          msgId: row[0],
          readCt: row[1],
          enqueuedAt: row[2],
          vt: row[3],
          message: row[4]
        }));
      } else {
        pgmqMessages = [];
      }
    } catch (e) {
      console.error(e);
    } finally {
      pgmqLoading = false;
    }
  }

  async function createPgmqQueue() {
    if (!activeConnection || activeConnection.kind !== 'postgres' || !pgmqNewQueueName.trim()) return;
    pgmqLoading = true;
    try {
      let q = pgmqNewQueueUnlogged 
        ? `SELECT pgmq.create_unlogged('${escapeSqlVal(pgmqNewQueueName)}');` 
        : `SELECT pgmq.create('${escapeSqlVal(pgmqNewQueueName)}');`;
      let res = await api.dbRunQuery('postgres', activeConnection.url, q);
      if (res.error) {
        alert(`Failed to create queue: ${res.error}`);
      } else {
        await loadPgmqQueues();
        showCreateQueueModal = false;
        pgmqNewQueueName = '';
        pgmqNewQueueUnlogged = false;
      }
    } catch (e) {
      alert(`Error creating queue: ${String(e)}`);
    } finally {
      pgmqLoading = false;
    }
  }

  async function sendPgmqMessage() {
    if (!activeConnection || activeConnection.kind !== 'postgres' || !selectedPgmqQueue || !pgmqSendMessageValue.trim()) return;
    pgmqLoading = true;
    try {
      // Validate JSON first
      try {
        JSON.parse(pgmqSendMessageValue);
      } catch (e) {
        throw new Error(`Invalid JSON payload: ${String(e)}`);
      }

      let q = `SELECT * FROM pgmq.send('${escapeSqlVal(selectedPgmqQueue)}', '${escapeSqlVal(pgmqSendMessageValue)}'::jsonb);`;
      let res = await api.dbRunQuery('postgres', activeConnection.url, q);
      if (res.error) {
        alert(`Failed to send message: ${res.error}`);
      } else {
        showSendPgmqMessageModal = false;
        pgmqSendMessageValue = '{\n  "hello": "world"\n}';
        await loadPgmqQueueData();
      }
    } catch (e) {
      alert(`Error sending message: ${String(e)}`);
    } finally {
      pgmqLoading = false;
    }
  }

  async function archivePgmqMessage(msgId: any) {
    if (!activeConnection || activeConnection.kind !== 'postgres' || !selectedPgmqQueue) return;
    try {
      let q = `SELECT pgmq.archive('${escapeSqlVal(selectedPgmqQueue)}', ${msgId});`;
      let res = await api.dbRunQuery('postgres', activeConnection.url, q);
      if (res.error) {
        alert(`Failed to archive message: ${res.error}`);
      } else {
        await loadPgmqQueueData();
      }
    } catch (e) {
      alert(`Error archiving message: ${String(e)}`);
    }
  }

  async function deletePgmqMessage(msgId: any) {
    if (!activeConnection || activeConnection.kind !== 'postgres' || !selectedPgmqQueue) return;
    if (!confirm(`Permanently delete message ${msgId}?`)) return;
    try {
      let q = `SELECT pgmq.delete('${escapeSqlVal(selectedPgmqQueue)}', ${msgId});`;
      let res = await api.dbRunQuery('postgres', activeConnection.url, q);
      if (res.error) {
        alert(`Failed to delete message: ${res.error}`);
      } else {
        await loadPgmqQueueData();
      }
    } catch (e) {
      alert(`Error deleting message: ${String(e)}`);
    }
  }

  async function readPgmqMessage() {
    if (!activeConnection || activeConnection.kind !== 'postgres' || !selectedPgmqQueue) return;
    pgmqLoading = true;
    try {
      // Read 1 message with a Visibility Timeout of 30 seconds
      let q = `SELECT msg_id, read_ct, enqueued_at::text, vt::text, message::text FROM pgmq.read('${escapeSqlVal(selectedPgmqQueue)}', 30, 1);`;
      let res = await api.dbRunQuery('postgres', activeConnection.url, q);
      if (res.error) {
        alert(`Failed to read message: ${res.error}`);
      } else if (res.rows && res.rows.length > 0) {
        let msg = res.rows[0];
        alert(`Read Message Successfully!\n\nID: ${msg[0]}\nRead Count: ${msg[1]}\nEnqueued: ${msg[2]}\nVT: ${msg[3]}\nPayload: ${msg[4]}`);
        await loadPgmqQueueData();
      } else {
        alert('No available (visible) messages in the queue.');
      }
    } catch (e) {
      alert(`Error reading message: ${String(e)}`);
    } finally {
      pgmqLoading = false;
    }
  }

  async function dropPgmqQueue(queueName: string) {
    if (!activeConnection || activeConnection.kind !== 'postgres') return;
    if (!confirm(`Are you sure you want to drop queue "${queueName}"? All messages will be permanently lost.`)) return;
    pgmqLoading = true;
    try {
      let q = `SELECT pgmq.drop_queue('${escapeSqlVal(queueName)}');`;
      let res = await api.dbRunQuery('postgres', activeConnection.url, q);
      if (res.error) {
        alert(`Failed to drop queue: ${res.error}`);
      } else {
        if (selectedPgmqQueue === queueName) {
          selectedPgmqQueue = null;
          pgmqMetrics = null;
          pgmqMessages = [];
        }
        await loadPgmqQueues();
      }
    } catch (e) {
      alert(`Error dropping queue: ${String(e)}`);
    } finally {
      pgmqLoading = false;
    }
  }

  // Postgres legacy backups helpers
  async function backupNow() {
    backingUp = true;
    try {
      await api.dbBackupNow();
      backups = await api.dbBackups();
    } catch (e) {
      alert(`Backup failed: ${String(e)}`);
    } finally {
      backingUp = false;
    }
  }

  async function restoreBackup(b: BackupFile) {
    if (!confirm(`Are you sure you want to restore from ${b.name}? This will overwrite your current database.`)) return;
    try {
      await api.dbRestore(b.name);
      alert('Database restored successfully.');
    } catch (e) {
      alert(`Restore failed: ${String(e)}`);
    }
  }

  async function deleteBackup(b: BackupFile) {
    if (!confirm(`Delete backup ${b.name}?`)) return;
    try {
      await api.dbDeleteBackup(b.name);
      backups = await api.dbBackups();
    } catch (e) {
      alert(`Delete failed: ${String(e)}`);
    }
  }

  function formatBytes(bytes: number) {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }
</script>

<div class="studio-container animate-fade-in">
  <!-- Top Switcher / Header -->
  <header class="studio-header">
    <div class="brand-section">
      <Icon name="database" size={20} class="text-crush-orange" />
      <span class="studio-title">Database Studio</span>
      {#if activeConnection}
        <span class="active-badge animate-slide-up">
          <span class="pulse-dot"></span>
          {activeConnection.name}
        </span>
      {/if}
    </div>

    <div class="actions-section">
      {#if activeConnection}
        <button class="switcher-btn" onclick={() => switcherOpen = !switcherOpen}>
          <span>Switch Connection</span>
          <Icon name="trendDown" size={14} />
        </button>
        <button class="disconnect-btn" onclick={() => activeConnection = null}>
          Disconnect
        </button>
      {/if}
    </div>

    <!-- Dropdown Selector -->
    {#if switcherOpen}
      <div class="switcher-dropdown crush-card">
        <h3>Running native services</h3>
        <div class="service-list stagger">
          {#each nativeServices as svc}
            <button class="dropdown-item" onclick={() => connectToService(svc)}>
              <TechIcon name={svc.kind} size={18} />
              <div class="item-text">
                <span class="svc-name">{svc.name}</span>
                <span class="svc-port">Port {svc.port}</span>
              </div>
            </button>
          {/each}
        </div>
        <div class="dropdown-footer">
          <button class="custom-connect-btn" onclick={() => { showCustomConnect = true; switcherOpen = false; }}>
            Connect with custom URL...
          </button>
        </div>
      </div>
    {/if}
  </header>

  <!-- Connection Screen (If not connected) -->
  {#if !activeConnection}
    <div class="connect-screen">
      <div class="connect-card crush-card animate-slide-up">
        <h2>Connect a Database</h2>
        <p class="muted">Select a running native database service or connect using a custom URL.</p>
        
        {#if connecting}
          <div class="connecting-overlay">
            <span class="spinner"></span>
            <span>Connecting...</span>
          </div>
        {/if}

        {#if connectionError}
          <div class="error-banner">
            <Icon name="x" size={14} />
            <span>{connectionError}</span>
          </div>
        {/if}

        <div class="connect-grid stagger">
          {#each nativeServices as svc}
            <button class="grid-connect-btn" onclick={() => connectToService(svc)}>
              <TechIcon name={svc.kind} size={28} />
              <span class="grid-title">{svc.kind.toUpperCase()}</span>
              <span class="grid-subtitle">Port {svc.port}</span>
            </button>
          {/each}
        </div>

        <div class="divider">
          <span>or connect using URL</span>
        </div>

        <div class="custom-form">
          <div class="input-row">
            <select class="crush-input" bind:value={customKind}>
              <option value="postgres">PostgreSQL</option>
              <option value="mysql">MySQL</option>
              <option value="redis">Redis</option>
              <option value="mongodb">MongoDB</option>
            </select>
            <input 
              type="text" 
              class="crush-input flex-1" 
              placeholder="postgresql://postgres:postgres@localhost:5432/postgres" 
              bind:value={customUrl} 
            />
          </div>
          <button class="btn primary full-width" onclick={connectToCustom} disabled={connecting}>
            Connect
          </button>
        </div>
      </div>
    </div>
  {:else}
    <!-- Active Workspace -->
    <div class="studio-workspace">
      <!-- Navigation Tabs -->
      <div class="tabs-nav">
        {#if activeConnection.kind === 'postgres' || activeConnection.kind === 'mysql'}
          <button class="tab-btn" class:active={activeTab === 'data'} onclick={() => activeTab = 'data'}>
            <Icon name="database" size={14} /> Data Grid
          </button>
          <button class="tab-btn" class:active={activeTab === 'sql'} onclick={() => activeTab = 'sql'}>
            <Icon name="play" size={14} /> SQL Editor
          </button>
          <button class="tab-btn" class:active={activeTab === 'schema'} onclick={() => activeTab = 'schema'}>
            <Icon name="folder" size={14} /> Schema
          </button>
          {#if activeConnection.kind === 'postgres'}
            <button class="tab-btn" class:active={activeTab === 'extensions'} onclick={() => activeTab = 'extensions'}>
              <Icon name="sparkles" size={14} /> Extensions
            </button>
            <button class="tab-btn" class:active={activeTab === 'performance'} onclick={() => activeTab = 'performance'}>
              <Icon name="activity" size={14} /> Performance
            </button>
            <button class="tab-btn" class:active={activeTab === 'security'} onclick={() => activeTab = 'security'}>
              <Icon name="settings" size={14} /> Security
            </button>
            <button class="tab-btn" class:active={activeTab === 'backups'} onclick={() => activeTab = 'backups'}>
              <Icon name="disk" size={14} /> Backups
            </button>
            <button class="tab-btn" class:active={activeTab === 'pgmq'} onclick={() => activeTab = 'pgmq'}>
              <Icon name="mail" size={14} /> Queues (PGMQ)
            </button>
          {/if}
        {:else}
          <!-- redis / mongo tabs -->
          {#if activeConnection.kind === 'redis'}
            <button class="tab-btn active">
              <Icon name="database" size={14} /> Keys Browser
            </button>
          {:else if activeConnection.kind === 'mongodb'}
            <button class="tab-btn active">
              <Icon name="database" size={14} /> Collections
            </button>
          {/if}
        {/if}
      </div>

      <!-- Tab Content Panel -->
      <div class="tab-panel">
        
        <!-- Tab: SQL / SQL Engines - Data Grid -->
        {#if activeTab === 'data'}
          <div class="data-grid-layout">
            <!-- Tables sidebar -->
            <aside class="tables-sidebar crush-card">
              <div class="sidebar-search">
                <input type="text" class="crush-input" placeholder="Search tables..." bind:value={tableSearch} />
              </div>
              <div class="sidebar-list">
                {#each filteredTables as t}
                  <button class="sidebar-item" class:active={selectedTable?.name === t.name} onclick={() => selectTable(t)}>
                    <Icon name="folder" size={14} />
                    <span>{t.name}</span>
                  </button>
                {/each}
              </div>
            </aside>

            <!-- Table data workspace -->
            <main class="data-workspace">
              {#if !selectedTable}
                <div class="empty-workspace">
                  <Icon name="database" size={32} class="muted" />
                  <h3>No table selected</h3>
                  <p class="muted">Select a table from the sidebar to browse and edit its data.</p>
                </div>
              {:else}
                <div class="workspace-controls">
                  <div class="left-controls">
                    <input 
                      type="text" 
                      class="crush-input search-input" 
                      placeholder="Filter rows (fuzzy search)..." 
                      bind:value={filterText}
                      onkeydown={(e) => e.key === 'Enter' && loadTableData(selectedTable!)}
                    />
                    <button class="btn" onclick={() => loadTableData(selectedTable!)}>
                      <Icon name="refresh" size={14} />
                    </button>
                  </div>
                  
                  <div class="right-controls">
                    <button class="btn primary" onclick={() => showInsertModal = true}>
                      <Icon name="sparkles" size={14} /> Insert Row
                    </button>
                  </div>
                </div>

                <div class="data-table-container crush-card">
                  {#if dataLoading}
                    <div class="loader-overlay">
                      <span class="spinner"></span>
                      <span>Loading table data...</span>
                    </div>
                  {:else}
                    <table class="grid-table">
                      <thead>
                        <tr>
                          <th></th> <!-- Actions -->
                          {#each columns as col}
                            <th onclick={() => {
                              if (tableSort?.column === col.name) {
                                tableSort = { column: col.name, desc: !tableSort.desc };
                              } else {
                                tableSort = { column: col.name, desc: false };
                              }
                              loadTableData(selectedTable!);
                            }}>
                              <div class="th-content">
                                <span>{col.name}</span>
                                <span class="col-type">{col.type}</span>
                                {#if tableSort?.column === col.name}
                                  <Icon name={tableSort.desc ? 'trendDown' : 'trendUp'} size={12} />
                                {/if}
                              </div>
                            </th>
                          {/each}
                        </tr>
                      </thead>
                      <tbody>
                        {#each rows as row, rIdx}
                          <tr>
                            <td class="action-cell">
                              <button class="delete-row-btn" onclick={() => deleteRow(row)}>
                                <Icon name="trash" size={14} />
                              </button>
                            </td>
                            {#each columns as col, cIdx}
                              <td 
                                class="data-cell"
                                ondblclick={() => editingCell = { rowIdx: rIdx, colName: col.name, value: String(row[cIdx] ?? '') }}
                              >
                                {#if editingCell?.rowIdx === rIdx && editingCell?.colName === col.name}
                                  <input 
                                    type="text" 
                                    class="cell-editor-input"
                                    bind:value={editingCell.value}
                                    onkeydown={(e) => {
                                      if (e.key === 'Enter') saveCellEdit(rIdx, col.name, editingCell!.value, row);
                                      if (e.key === 'Escape') editingCell = null;
                                    }}
                                    onblur={() => saveCellEdit(rIdx, col.name, editingCell!.value, row)}
                                    use:focusOnMount
                                  />
                                {:else}
                                  <span class:null-val={row[cIdx] === null}>
                                    {row[cIdx] === null ? 'NULL' : row[cIdx]}
                                  </span>
                                {/if}
                              </td>
                            {/each}
                          </tr>
                        {/each}
                      </tbody>
                    </table>
                  {/if}
                </div>

                <!-- Pagination footer -->
                <footer class="pagination-footer">
                  <div class="page-info">
                    <span>Showing {rows.length} rows of {dataTotalRows}</span>
                  </div>
                  <div class="page-controls">
                    <button 
                      class="btn sm" 
                      disabled={dataPage === 0} 
                      onclick={() => { dataPage--; loadTableData(selectedTable!); }}
                    >
                      Prev
                    </button>
                    <span class="page-indicator">Page {dataPage + 1}</span>
                    <button 
                      class="btn sm" 
                      disabled={(dataPage + 1) * dataLimit >= dataTotalRows}
                      onclick={() => { dataPage++; loadTableData(selectedTable!); }}
                    >
                      Next
                    </button>
                  </div>
                </footer>
              {/if}
            </main>
          </div>
        {/if}

        <!-- Tab: SQL Editor -->
        {#if activeTab === 'sql'}
          <div class="sql-editor-layout">
            <div class="editor-pane">
              <div class="editor-header">
                <h3>Query Editor</h3>
                <button class="btn primary" onclick={runSQL} disabled={sqlLoading}>
                  <Icon name="play" size={14} /> Run Query
                </button>
              </div>
              <textarea class="sql-textarea crush-input" bind:value={sqlQuery}></textarea>

              {#if sqlError}
                <div class="error-panel">
                  <h4>Error running query</h4>
                  <pre>{sqlError}</pre>
                </div>
              {/if}

              {#if sqlResults}
                <div class="query-stats">
                  <span>Affected rows: {sqlResults.affected}</span>
                  <span>Execution time: {sqlResults.duration_ms} ms</span>
                </div>
                
                <div class="sql-results-grid crush-card">
                  <table class="grid-table">
                    <thead>
                      <tr>
                        {#each sqlResults.columns as col}
                          <th>{col}</th>
                        {/each}
                      </tr>
                    </thead>
                    <tbody>
                      {#each sqlResults.rows as row}
                        <tr>
                          {#each row as cell}
                            <td>{cell === null ? 'NULL' : cell}</td>
                          {/each}
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>
              {/if}
            </div>

            <!-- History Panel -->
            <aside class="history-sidebar crush-card">
              <h3>History</h3>
              <div class="history-list">
                {#each sqlHistory as item}
                  <button class="history-item" onclick={() => sqlQuery = item}>
                    <pre>{item}</pre>
                  </button>
                {/each}
              </div>
            </aside>
          </div>
        {/if}

        <!-- Tab: Schema -->
        {#if activeTab === 'schema'}
          <div class="schema-layout stagger">
            <h2>Database Schema Overview</h2>
            <div class="ctable">
              <div class="crow chead">
                <span>Table Name</span>
                <span>Size</span>
                <span>Index Size</span>
                <span>Live Rows</span>
              </div>
              {#each schemaTables as row}
                <div class="crow">
                  <span class="mono bold">{row[0]}</span>
                  <span>{row[1]}</span>
                  <span>{row[2]}</span>
                  <span>{row[3]}</span>
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Tab: Extensions (Postgres only) -->
        {#if activeTab === 'extensions'}
          <div class="extensions-layout stagger">
            <h2>PostgreSQL Extensions</h2>
            <p class="muted">Enable vectors, stats, and search libraries with one click.</p>
            <div class="extensions-grid">
              {#each extensions as ext}
                <div class="ext-card crush-card">
                  <div class="ext-head">
                    <span class="ext-name">{ext.name}</span>
                    <span class="ext-ver">{ext.version}</span>
                  </div>
                  <p class="ext-desc">{ext.description || 'No description available.'}</p>
                  <button 
                    class="btn btn-ext" 
                    class:primary={!ext.installed} 
                    onclick={() => toggleExtension(ext)}
                  >
                    {ext.installed ? 'Installed' : 'Enable'}
                  </button>
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Tab: Performance (Postgres only) -->
        {#if activeTab === 'performance'}
          <div class="performance-layout stagger">
            <h2>Performance Advisor</h2>
            <div class="explain-panel crush-card">
              <h3>EXPLAIN Query Advisor</h3>
              <textarea class="crush-input" placeholder="SELECT * FROM my_table WHERE id = 1" bind:value={explainQuery}></textarea>
              <button class="btn primary" onclick={explain}>Analyze Plan</button>
              {#if explainResult}
                <pre class="explain-result">{explainResult}</pre>
              {/if}
            </div>

            <div class="slow-queries-panel crush-card">
              <h3>Top 10 Slowest Queries</h3>
              <div class="ctable">
                <div class="crow chead">
                  <span>Query Pattern</span>
                  <span>Calls</span>
                  <span>Mean Time (ms)</span>
                </div>
                {#each slowQueries as row}
                  <div class="crow">
                    <span class="mono text-xs">{row[0]}</span>
                    <span>{row[1]}</span>
                    <span>{parseFloat(row[3]).toFixed(2)}</span>
                  </div>
                {/each}
              </div>
            </div>
          </div>
        {/if}

        <!-- Tab: Security (Postgres only) -->
        {#if activeTab === 'security'}
          <div class="security-layout stagger">
            <h2>Roles & Row Level Security</h2>
            <div class="roles-section crush-card">
              <h3>Database Roles</h3>
              <div class="ctable">
                <div class="crow chead">
                  <span>Role Name</span>
                  <span>Superuser</span>
                  <span>Can Login</span>
                </div>
                {#each pgRoles as role}
                  <div class="crow">
                    <span class="mono">{role[0]}</span>
                    <span>{role[1] ? 'Yes' : 'No'}</span>
                    <span>{role[5] ? 'Yes' : 'No'}</span>
                  </div>
                {/each}
              </div>
            </div>

            <div class="rls-section crush-card">
              <h3>RLS Policies</h3>
              <div class="ctable">
                <div class="crow chead">
                  <span>Table</span>
                  <span>Policy</span>
                  <span>Command</span>
                  <span>Roles</span>
                </div>
                {#each rlsPolicies as pol}
                  <div class="crow">
                    <span>{pol[1]}</span>
                    <span class="mono">{pol[2]}</span>
                    <span>{pol[3]}</span>
                    <span>{pol[4]}</span>
                  </div>
                {/each}
              </div>
            </div>
          </div>
        {/if}

        <!-- Tab: Backups -->
        {#if activeTab === 'backups'}
          <div class="backups-layout animate-fade-in">
            <header class="ph">
              <h2>Database Backups</h2>
              <button class="btn primary" onclick={backupNow} disabled={backingUp}>
                {backingUp ? 'Backing up...' : 'Backup Now'}
              </button>
            </header>

            {#if backups.length === 0}
              <div class="empty-box">
                <Icon name="database" size={26} />
                <p class="empty-title">No backups found</p>
                <p class="muted">Click "Backup Now" to create your first database backup.</p>
              </div>
            {:else}
              <div class="ctable">
                <div class="crow chead">
                  <span>File Name</span>
                  <span>Size</span>
                  <span>Actions</span>
                </div>
                {#each backups as b}
                  <div class="crow">
                    <span class="cname mono">{b.name}</span>
                    <span class="mono dim">{formatBytes(b.size)}</span>
                    <div class="actions">
                      <button class="ghost-btn sm" onclick={() => restoreBackup(b)}>
                        <Icon name="refresh" size={14} /> Restore
                      </button>
                      <button class="ghost-btn sm text-red" onclick={() => deleteBackup(b)}>
                        <Icon name="trash" size={14} />
                      </button>
                    </div>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        {/if}

        <!-- Tab: PGMQ Queues -->
        {#if activeTab === 'pgmq'}
          <div class="pgmq-layout animate-fade-in">
            {#if !pgmqEnabled}
              <div class="empty-box crush-card text-center p-8 stagger">
                <div class="empty-icon-wrapper mb-4">
                  <Icon name="mail" size={48} class="text-primary-400" />
                </div>
                <h3 class="text-lg font-bold mb-2">PGMQ is not enabled</h3>
                <p class="muted max-w-md mx-auto mb-6">
                  Postgres Message Queues (PGMQ) is a lightweight message queue extension. It lets you use PostgreSQL as a message queue with API parity to AWS SQS.
                </p>
                <button class="btn primary" onclick={enablePgmq} disabled={pgmqLoading}>
                  {pgmqLoading ? 'Enabling PGMQ...' : 'Enable PGMQ Extension'}
                </button>
              </div>
            {:else}
              <div class="queues-workspace-layout">
                <!-- Sidebar of Queues -->
                <aside class="queues-sidebar crush-card">
                  <div class="sidebar-header">
                    <h3>Queues</h3>
                    <button class="btn primary sm" onclick={() => showCreateQueueModal = true}>
                      <Icon name="sparkles" size={12} /> New
                    </button>
                  </div>
                  <div class="sidebar-list mt-2">
                    {#if pgmqQueues.length === 0}
                      <p class="muted p-4 text-center">No queues found</p>
                    {:else}
                      {#each pgmqQueues as q}
                        <div class="sidebar-item" class:active={selectedPgmqQueue === q.name} role="button" tabindex="0" onclick={() => selectPgmqQueue(q.name)} onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') selectPgmqQueue(q.name); }}>
                          <Icon name="mail" size={14} />
                          <span class="queue-name text-ellipsis">{q.name}</span>
                          <button class="ghost-btn sm text-red ml-auto delete-queue-icon-btn" onclick={(e) => { e.stopPropagation(); dropPgmqQueue(q.name); }}>
                            <Icon name="trash" size={12} />
                          </button>
                        </div>
                      {/each}
                    {/if}
                  </div>
                </aside>

                <!-- Queue Data Workspace -->
                <main class="queue-data-workspace">
                  {#if !selectedPgmqQueue}
                    <div class="empty-workspace crush-card">
                      <Icon name="mail" size={32} class="muted" />
                      <h3>No queue selected</h3>
                      <p class="muted">Select a queue from the sidebar or create a new one to view and send messages.</p>
                    </div>
                  {:else}
                    <!-- Metrics Card -->
                    <div class="queue-header-card crush-card animate-slide-up">
                      <div class="queue-info-row">
                        <div>
                          <h2>{selectedPgmqQueue}</h2>
                          {#if pgmqMetrics}
                            <div class="metrics-grid mt-2">
                              <div class="metric-item">
                                <span class="lbl">Length</span>
                                <span class="val mono">{pgmqMetrics.queue_length}</span>
                              </div>
                              <div class="metric-item">
                                <span class="lbl">Total</span>
                                <span class="val mono">{pgmqMetrics.total_messages}</span>
                              </div>
                              <div class="metric-item">
                                <span class="lbl">Oldest Age</span>
                                <span class="val mono">{pgmqMetrics.oldest_msg_age_sec}s</span>
                              </div>
                              <div class="metric-item">
                                <span class="lbl">Newest Age</span>
                                <span class="val mono">{pgmqMetrics.newest_msg_age_sec}s</span>
                              </div>
                            </div>
                          {/if}
                        </div>
                        <div class="actions" style="display: flex; gap: 8px;">
                          <button class="btn" onclick={loadPgmqQueueData}>
                            <Icon name="refresh" size={14} /> Refresh
                          </button>
                          <button class="btn" onclick={readPgmqMessage}>
                            <Icon name="play" size={14} /> Read/Pop
                          </button>
                          <button class="btn primary" onclick={() => showSendPgmqMessageModal = true}>
                            <Icon name="sparkles" size={14} /> Send Msg
                          </button>
                        </div>
                      </div>
                    </div>

                    <!-- Messages list -->
                    <div class="queue-messages-card crush-card mt-4 animate-slide-up">
                      <h3>Messages (Latest 100)</h3>
                      <div class="ctable mt-2">
                        <div class="crow chead msg-row">
                          <span>Msg ID</span>
                          <span>Enqueued At</span>
                          <span>Read Ct</span>
                          <span>VT</span>
                          <span>Payload (JSON)</span>
                          <span>Actions</span>
                        </div>
                        {#if pgmqMessages.length === 0}
                          <div class="crow text-center muted p-4">
                            <span>No messages in queue</span>
                          </div>
                        {:else}
                          {#each pgmqMessages as msg}
                            <div class="crow msg-row">
                              <span class="mono">{msg.msgId}</span>
                              <span class="mono dim">{msg.enqueuedAt}</span>
                              <span class="mono">{msg.readCt}</span>
                              <span class="mono dim">{msg.vt}</span>
                              <span class="mono payload-cell" title={msg.message}>{msg.message}</span>
                              <div class="actions" style="display: flex; gap: 4px;">
                                <button class="ghost-btn sm" onclick={() => archivePgmqMessage(msg.msgId)} title="Archive Message">
                                  Archive
                                </button>
                                <button class="ghost-btn sm text-red" onclick={() => deletePgmqMessage(msg.msgId)} title="Delete Message">
                                  <Icon name="trash" size={12} />
                                </button>
                              </div>
                            </div>
                          {/each}
                        {/if}
                      </div>
                    </div>
                  {/if}
                </main>
              </div>
            {/if}
          </div>
        {/if}

        <!-- Redis Keys Tab -->
        {#if activeTab === 'redis-keys'}
          <div class="redis-layout">
            <!-- Sidebar list keys -->
            <aside class="keys-sidebar crush-card">
              <div class="sidebar-search flex gap-1">
                <input type="text" class="crush-input flex-1" placeholder="KEYS pattern..." bind:value={redisPattern} />
                <button class="btn" onclick={loadRedisKeys}>
                  <Icon name="refresh" size={14} />
                </button>
              </div>
              <div class="sidebar-list">
                {#each redisKeys as key}
                  <button class="sidebar-item" class:active={selectedRedisKey?.key === key.key} onclick={() => selectRedisKey(key)}>
                    <span class="key-badge" class:str={key.kind === 'string'}>{key.kind.substring(0, 3)}</span>
                    <span class="key-name text-ellipsis">{key.key}</span>
                  </button>
                {/each}
              </div>
              <button class="btn primary w-full mt-2" onclick={() => showAddRedisKeyModal = true}>
                Add Key
              </button>
            </aside>

            <!-- Key view pane -->
            <main class="key-workspace">
              {#if !selectedRedisKey}
                <div class="empty-workspace">
                  <Icon name="database" size={32} class="muted" />
                  <h3>No key selected</h3>
                  <p class="muted">Select a key from the sidebar to view/edit value and TTL.</p>
                </div>
              {:else}
                <div class="key-card crush-card animate-slide-up">
                  <div class="key-header">
                    <h2>{selectedRedisKey.key}</h2>
                    <button class="delete-row-btn" onclick={() => deleteRedisKey(selectedRedisKey!.key)}>
                      <Icon name="trash" size={16} />
                    </button>
                  </div>
                  <div class="key-metadata">
                    <span>Type: <strong>{redisKeyType}</strong></span>
                    <span>TTL (secs): <input type="number" class="crush-input ttl-input inline" bind:value={redisKeyTtl} /></span>
                  </div>
                  <div class="key-editor">
                    <label for="redis-key-val">Value</label>
                    <textarea id="redis-key-val" class="crush-input key-val-textarea" bind:value={redisKeyValue}></textarea>
                  </div>
                  <button class="btn primary" onclick={saveRedisKey}>Save Key</button>
                </div>
              {/if}
            </main>
          </div>
        {/if}

        <!-- MongoDB tab -->
        {#if activeTab === 'mongo-colls'}
          <div class="mongo-layout">
            <aside class="mongo-sidebar crush-card">
              <div class="db-switcher">
                <label for="mongo-db-select">Database</label>
                <select id="mongo-db-select" class="crush-input" bind:value={selectedMongoDb} onchange={loadMongoCollections}>
                  {#each mongoDbs as db}
                    <option value={db}>{db}</option>
                  {/each}
                </select>
              </div>
              <div class="sidebar-list">
                {#each mongoColls as coll}
                  <button class="sidebar-item" class:active={selectedMongoColl === coll} onclick={() => selectMongoCollection(coll)}>
                    <Icon name="folder" size={14} />
                    <span>{coll}</span>
                  </button>
                {/each}
              </div>
            </aside>

            <!-- Mongo docs panel -->
            <main class="mongo-workspace">
              {#if !selectedMongoColl}
                <div class="empty-workspace">
                  <Icon name="database" size={32} class="muted" />
                  <h3>No collection selected</h3>
                  <p class="muted">Select a collection from the sidebar to view documents.</p>
                </div>
              {:else}
                <div class="workspace-controls">
                  <div class="left-controls">
                    <input 
                      type="text" 
                      class="crush-input search-input" 
                      placeholder="Query filter, e.g. &#123;&quot;status&quot;: &quot;active&quot;&#125;" 
                      bind:value={mongoFilter}
                      onkeydown={(e) => e.key === 'Enter' && loadMongoDocs()}
                    />
                    <button class="btn" onclick={loadMongoDocs}>
                      <Icon name="refresh" size={14} />
                    </button>
                  </div>

                  <div class="right-controls">
                    <button class="btn primary" onclick={() => showInsertMongoDocModal = true}>
                      Insert Document
                    </button>
                  </div>
                </div>

                <div class="mongo-docs-list stagger">
                  {#each mongoDocs as doc, idx}
                    <div class="doc-card crush-card">
                      <div class="doc-header">
                        <span class="mono text-xs">ID: {doc._id?.$oid || doc._id}</span>
                        <div class="actions">
                          {#if editingMongoDoc?.index === idx}
                            <button class="btn sm primary" onclick={() => saveMongoDoc(idx)}>Save</button>
                            <button class="btn sm" onclick={() => editingMongoDoc = null}>Cancel</button>
                          {:else}
                            <button class="btn sm" onclick={() => editingMongoDoc = { index: idx, content: JSON.stringify(doc, null, 2) }}>Edit</button>
                          {/if}
                          <button class="delete-row-btn" onclick={() => deleteMongoDoc(doc)}>
                            <Icon name="trash" size={14} />
                          </button>
                        </div>
                      </div>

                      {#if editingMongoDoc?.index === idx}
                        <textarea class="crush-input doc-editor" bind:value={editingMongoDoc.content}></textarea>
                      {:else}
                        <pre class="doc-content">{JSON.stringify(doc, null, 2)}</pre>
                      {/if}
                    </div>
                  {/each}
                </div>
              {/if}
            </main>
          </div>
        {/if}

      </div>
    </div>
  {/if}
</div>

<!-- Modal Dialogs -->

<!-- Insert Row Modal (SQL Data Grid) -->
{#if showInsertModal}
  <div class="modal-backdrop" role="button" tabindex="-1" onclick={() => showInsertModal = false} onkeydown={(e) => { if (e.key === 'Escape') showInsertModal = false; }}>
    <div class="modal-card crush-card animate-slide-up" role="presentation" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
      <h3>Insert New Row</h3>
      <div class="modal-fields">
        {#each columns as col}
          <div class="field-row">
            <label>
              <span>{col.name} <span class="type-hint">({col.type})</span></span>
              <input type="text" class="crush-input" bind:value={insertFormValues[col.name]} placeholder={col.nullable ? 'NULL' : ''} />
            </label>
          </div>
        {/each}
      </div>
      <div class="modal-footer">
        <button class="btn" onclick={() => showInsertModal = false}>Cancel</button>
        <button class="btn primary" onclick={insertRow}>Insert</button>
      </div>
    </div>
  </div>
{/if}

<!-- Add Redis Key Modal -->
{#if showAddRedisKeyModal}
  <div class="modal-backdrop" role="button" tabindex="-1" onclick={() => showAddRedisKeyModal = false} onkeydown={(e) => { if (e.key === 'Escape') showAddRedisKeyModal = false; }}>
    <div class="modal-card crush-card animate-slide-up" role="presentation" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
      <h3>Add Redis Key</h3>
      <div class="modal-fields">
        <div class="field-row">
          <label for="redis-new-key-name">Key Name</label>
          <input id="redis-new-key-name" type="text" class="crush-input" bind:value={redisNewKeyName} placeholder="user:123" />
        </div>
        <div class="field-row">
          <label for="redis-new-key-val">Value</label>
          <textarea id="redis-new-key-val" class="crush-input" bind:value={redisNewKeyValue} placeholder="value"></textarea>
        </div>
      </div>
      <div class="modal-footer">
        <button class="btn" onclick={() => showAddRedisKeyModal = false}>Cancel</button>
        <button class="btn primary" onclick={addRedisKey}>Create</button>
      </div>
    </div>
  </div>
{/if}

<!-- Insert Mongo Doc Modal -->
{#if showInsertMongoDocModal}
  <div class="modal-backdrop" role="button" tabindex="-1" onclick={() => showInsertMongoDocModal = false} onkeydown={(e) => { if (e.key === 'Escape') showInsertMongoDocModal = false; }}>
    <div class="modal-card crush-card animate-slide-up" role="presentation" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
      <h3>Insert MongoDB Document</h3>
      <div class="modal-fields">
        <textarea aria-label="MongoDB Document JSON" class="crush-input doc-editor" bind:value={insertMongoDocValue}></textarea>
      </div>
      <div class="modal-footer">
        <button class="btn" onclick={() => showInsertMongoDocModal = false}>Cancel</button>
        <button class="btn primary" onclick={insertMongoDoc}>Insert</button>
      </div>
    </div>
  </div>
{/if}

<!-- Create PGMQ Queue Modal -->
{#if showCreateQueueModal}
  <div class="modal-backdrop" role="button" tabindex="-1" onclick={() => showCreateQueueModal = false} onkeydown={(e) => { if (e.key === 'Escape') showCreateQueueModal = false; }}>
    <div class="modal-card crush-card animate-slide-up" role="presentation" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
      <h3>Create PGMQ Queue</h3>
      <div class="modal-fields">
        <div class="field-row">
          <label for="pgmq-new-qname">Queue Name</label>
          <input id="pgmq-new-qname" type="text" class="crush-input" bind:value={pgmqNewQueueName} placeholder="my-jobs" />
        </div>
        <div class="field-row checkbox-row">
          <label style="display: flex; align-items: center; gap: 8px; cursor: pointer;">
            <input type="checkbox" bind:checked={pgmqNewQueueUnlogged} /> Unlogged Queue (Faster, but not crash-safe)
          </label>
        </div>
      </div>
      <div class="modal-footer">
        <button class="btn" onclick={() => showCreateQueueModal = false}>Cancel</button>
        <button class="btn primary" onclick={createPgmqQueue}>Create</button>
      </div>
    </div>
  </div>
{/if}

<!-- Send PGMQ Message Modal -->
{#if showSendPgmqMessageModal}
  <div class="modal-backdrop" role="button" tabindex="-1" onclick={() => showSendPgmqMessageModal = false} onkeydown={(e) => { if (e.key === 'Escape') showSendPgmqMessageModal = false; }}>
    <div class="modal-card crush-card animate-slide-up" role="presentation" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
      <h3>Send Message to {selectedPgmqQueue}</h3>
      <div class="modal-fields">
        <label for="pgmq-msg-text">Message Payload (JSON)</label>
        <textarea id="pgmq-msg-text" class="crush-input doc-editor" bind:value={pgmqSendMessageValue}></textarea>
      </div>
      <div class="modal-footer">
        <button class="btn" onclick={() => showSendPgmqMessageModal = false}>Cancel</button>
        <button class="btn primary" onclick={sendPgmqMessage}>Send</button>
      </div>
    </div>
  </div>
{/if}

<style>
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
    position: relative;
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
    border: 1px solid transparent;
    color: var(--color-crush-text-muted);
    font-size: 12px;
    cursor: pointer;
    padding: 6px 10px;
  }

  .disconnect-btn:hover {
    color: var(--color-crush-red);
  }

  .switcher-dropdown {
    position: absolute;
    top: calc(100% + 4px);
    right: 24px;
    width: 280px;
    padding: 12px;
    z-index: 100;
    box-shadow: 0 10px 25px rgba(0,0,0,0.5);
  }

  .switcher-dropdown h3 {
    font-size: 11px;
    text-transform: uppercase;
    color: var(--color-crush-text-muted);
    margin: 0 0 8px;
    letter-spacing: 0.05em;
  }

  .service-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .dropdown-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px;
    background: none;
    border: 1px solid transparent;
    border-radius: 6px;
    cursor: pointer;
    text-align: left;
    color: var(--color-crush-text);
  }

  .dropdown-item:hover {
    background: var(--color-crush-surface);
    border-color: var(--color-crush-border);
  }

  .item-text {
    display: flex;
    flex-direction: column;
  }

  .svc-name {
    font-size: 12px;
    font-weight: 500;
  }

  .svc-port {
    font-size: 10px;
    color: var(--color-crush-text-muted);
  }

  .dropdown-footer {
    border-top: 1px solid var(--color-crush-border);
    margin-top: 8px;
    padding-top: 8px;
  }

  .custom-connect-btn {
    width: 100%;
    background: none;
    border: none;
    color: var(--color-crush-text-muted);
    font-size: 12px;
    text-align: center;
    cursor: pointer;
    padding: 4px;
  }

  .custom-connect-btn:hover {
    color: var(--color-crush-text);
  }

  /* Connection screen */
  .connect-screen {
    display: flex;
    align-items: center;
    justify-content: center;
    flex: 1;
    padding: 24px;
    background: var(--color-crush-black);
  }

  .connect-card {
    width: 100%;
    max-width: 480px;
    padding: 32px;
    position: relative;
  }

  .connect-card h2 {
    font-size: 20px;
    font-weight: 600;
    margin: 0 0 6px;
  }

  .muted {
    color: var(--color-crush-text-muted);
    font-size: 13px;
  }

  .connecting-overlay {
    position: absolute;
    inset: 0;
    background: rgba(0,0,0,0.7);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    border-radius: inherit;
    z-index: 10;
  }

  .spinner {
    width: 24px;
    height: 24px;
    border: 2px solid var(--color-crush-border);
    border-top-color: var(--color-crush-orange);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .error-banner {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.2);
    border-radius: 8px;
    color: var(--color-crush-red);
    font-size: 12px;
    margin-top: 14px;
  }

  .connect-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 12px;
    margin-top: 24px;
  }

  .grid-connect-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 18px;
    background: var(--color-crush-surface);
    border: 1px solid var(--color-crush-border);
    border-radius: 10px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .grid-connect-btn:hover {
    border-color: #333;
    background: rgba(255,255,255,0.02);
  }

  .grid-title {
    font-size: 11px;
    font-weight: 600;
    margin-top: 8px;
    color: var(--color-crush-text);
  }

  .grid-subtitle {
    font-size: 10px;
    color: var(--color-crush-text-muted);
  }

  .divider {
    display: flex;
    align-items: center;
    text-align: center;
    color: var(--color-crush-text-muted);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin: 20px 0;
  }

  .divider::before, .divider::after {
    content: '';
    flex: 1;
    border-bottom: 1px solid var(--color-crush-border);
  }

  .divider:not(:empty)::before { margin-right: .5em; }
  .divider:not(:empty)::after { margin-left: .5em; }

  .custom-form {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .input-row {
    display: flex;
    gap: 8px;
  }

  .flex-1 { flex: 1; }

  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    background: var(--color-crush-surface);
    border: 1px solid var(--color-crush-border);
    color: var(--color-crush-text);
    border-radius: 8px;
    padding: 8px 14px;
    font-size: 13px;
    cursor: pointer;
    font-weight: 500;
  }

  .btn:hover:not(:disabled) {
    background: rgba(255,255,255,0.02);
    border-color: #333;
  }

  .btn.primary {
    background: var(--color-crush-primary);
    border-color: var(--color-crush-primary);
    color: var(--color-crush-on-primary);
  }

  .btn.primary:hover:not(:disabled) {
    background: var(--color-crush-primary-hover);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .full-width { width: 100%; }

  /* Workspace */
  .studio-workspace {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow: hidden;
  }

  .tabs-nav {
    display: flex;
    gap: 4px;
    background: var(--color-crush-dark);
    padding: 6px 16px 0;
    border-bottom: 1px solid var(--color-crush-border);
  }

  .tab-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 8px 16px;
    font-size: 12px;
    font-weight: 500;
    color: var(--color-crush-text-muted);
    border: none;
    border-bottom: 2px solid transparent;
    background: none;
    cursor: pointer;
  }

  .tab-btn:hover {
    color: var(--color-crush-text);
  }

  .tab-btn.active {
    color: var(--color-crush-text);
    border-bottom-color: var(--color-crush-primary);
  }

  .tab-panel {
    flex: 1;
    overflow: hidden;
    background: var(--color-crush-black);
    display: flex;
    flex-direction: column;
  }

  /* Data Grid Layout */
  .data-grid-layout {
    display: grid;
    grid-template-columns: 240px 1fr;
    height: 100%;
    overflow: hidden;
  }

  .tables-sidebar {
    border-right: 1px solid var(--color-crush-border);
    background: var(--color-crush-dark);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .sidebar-search {
    padding: 12px;
    border-bottom: 1px solid var(--color-crush-border);
  }

  .sidebar-list {
    flex: 1;
    overflow-y: auto;
    padding: 6px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .sidebar-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 10px;
    background: none;
    border: none;
    border-radius: 6px;
    text-align: left;
    color: var(--color-crush-text-muted);
    font-size: 12px;
    cursor: pointer;
  }

  .sidebar-item:hover, .sidebar-item.active {
    background: var(--color-crush-surface);
    color: var(--color-crush-text);
  }

  .data-workspace {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    padding: 16px;
    gap: 12px;
  }

  .empty-workspace {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    text-align: center;
    gap: 8px;
  }

  .empty-workspace h3 {
    margin: 8px 0 0;
    font-size: 15px;
    font-weight: 600;
  }

  .workspace-controls {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .left-controls {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .search-input {
    width: 240px;
  }

  .data-table-container {
    flex: 1;
    overflow: auto;
    border: 1px solid var(--color-crush-border);
    border-radius: 8px;
    background: var(--color-crush-dark);
    position: relative;
  }

  .loader-overlay {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    height: 100px;
  }

  .grid-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
  }

  .grid-table th, .grid-table td {
    padding: 8px 12px;
    text-align: left;
    border-bottom: 1px solid var(--color-crush-border);
    border-right: 1px solid var(--color-crush-border);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 240px;
  }

  .grid-table th {
    background: rgba(255,255,255,0.02);
    font-weight: 500;
    color: var(--color-crush-text-muted);
    cursor: pointer;
    user-select: none;
  }

  .th-content {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .col-type {
    font-size: 10px;
    color: var(--color-crush-text-muted);
    background: var(--color-crush-surface);
    padding: 1px 4px;
    border-radius: 4px;
  }

  .action-cell {
    width: 32px;
    text-align: center;
    padding: 4px !important;
  }

  .delete-row-btn {
    background: none;
    border: none;
    color: var(--color-crush-text-muted);
    cursor: pointer;
  }

  .delete-row-btn:hover {
    color: var(--color-crush-red);
  }

  .data-cell {
    cursor: cell;
  }

  .data-cell:hover {
    background: rgba(255,255,255,0.01);
  }

  .cell-editor-input {
    width: 100%;
    background: var(--color-crush-surface);
    border: 1px solid var(--color-crush-orange);
    color: var(--color-crush-text);
    padding: 2px 4px;
    font-size: 12px;
    border-radius: 4px;
    outline: none;
  }

  .null-val {
    color: var(--color-crush-text-muted);
    font-style: italic;
    font-size: 11px;
  }

  .pagination-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 12px;
    color: var(--color-crush-text-muted);
  }

  .page-controls {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  /* SQL Editor Layout */
  .sql-editor-layout {
    display: grid;
    grid-template-columns: 1fr 240px;
    height: 100%;
    overflow: hidden;
    padding: 16px;
    gap: 16px;
  }

  .editor-pane {
    display: flex;
    flex-direction: column;
    gap: 12px;
    height: 100%;
    overflow-y: auto;
  }

  .editor-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .editor-header h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
  }

  .sql-textarea {
    width: 100%;
    height: 140px;
    font-family: var(--font-mono);
    font-size: 13px;
    resize: none;
  }

  .error-panel {
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.2);
    border-radius: 8px;
    padding: 12px;
    color: var(--color-crush-red);
  }

  .error-panel h4 {
    margin: 0 0 6px;
    font-size: 13px;
    font-weight: 600;
  }

  .error-panel pre {
    margin: 0;
    font-family: var(--font-mono);
    font-size: 11px;
    white-space: pre-wrap;
  }

  .query-stats {
    display: flex;
    gap: 16px;
    font-size: 12px;
    color: var(--color-crush-text-muted);
  }

  .sql-results-grid {
    flex: 1;
    overflow: auto;
    border: 1px solid var(--color-crush-border);
  }

  .history-sidebar {
    background: var(--color-crush-dark);
    display: flex;
    flex-direction: column;
    padding: 12px;
    overflow: hidden;
  }

  .history-sidebar h3 {
    margin: 0 0 10px;
    font-size: 12px;
    text-transform: uppercase;
    color: var(--color-crush-text-muted);
    letter-spacing: 0.05em;
  }

  .history-list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .history-item {
    width: 100%;
    padding: 8px;
    background: var(--color-crush-surface);
    border: 1px solid var(--color-crush-border);
    border-radius: 6px;
    text-align: left;
    cursor: pointer;
    font-size: 11px;
    color: var(--color-crush-text-muted);
    overflow: hidden;
  }

  .history-item:hover {
    color: var(--color-crush-text);
    border-color: #333;
  }

  .history-item pre {
    margin: 0;
    white-space: nowrap;
    text-overflow: ellipsis;
    overflow: hidden;
  }

  /* Schema / table layout */
  .schema-layout {
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .schema-layout h2 {
    font-size: 18px;
    font-weight: 600;
    margin: 0;
  }

  /* Extensions (Postgres only) */
  .extensions-layout {
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    overflow-y: auto;
  }

  .extensions-layout h2 {
    font-size: 18px;
    font-weight: 600;
    margin: 0;
  }

  .extensions-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 14px;
  }

  .ext-card {
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .ext-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .ext-name {
    font-weight: 600;
    font-size: 14px;
  }

  .ext-ver {
    font-size: 11px;
    color: var(--color-crush-text-muted);
    background: var(--color-crush-surface);
    padding: 2px 6px;
    border-radius: 4px;
  }

  .ext-desc {
    font-size: 12px;
    color: var(--color-crush-text-muted);
    line-height: 1.4;
    flex: 1;
  }

  .btn-ext {
    margin-top: 8px;
  }

  /* Performance Tab */
  .performance-layout {
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 20px;
    overflow-y: auto;
  }

  .performance-layout h2 {
    font-size: 18px;
    font-weight: 600;
    margin: 0;
  }

  .explain-panel, .slow-queries-panel {
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .explain-panel h3, .slow-queries-panel h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
  }

  .explain-panel textarea {
    height: 80px;
    font-family: var(--font-mono);
  }

  .explain-result {
    margin-top: 12px;
    background: var(--color-crush-black);
    padding: 12px;
    border-radius: 6px;
    border: 1px solid var(--color-crush-border);
    font-family: var(--font-mono);
    font-size: 11px;
    overflow-x: auto;
  }

  /* Security Tab */
  .security-layout {
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 20px;
    overflow-y: auto;
  }

  .security-layout h2 {
    font-size: 18px;
    font-weight: 600;
    margin: 0;
  }

  .roles-section, .rls-section {
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .roles-section h3, .rls-section h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
  }

  /* Redis Layout */
  .redis-layout {
    display: grid;
    grid-template-columns: 260px 1fr;
    height: 100%;
    overflow: hidden;
  }

  .keys-sidebar {
    border-right: 1px solid var(--color-crush-border);
    background: var(--color-crush-dark);
    display: flex;
    flex-direction: column;
    padding: 12px;
    overflow: hidden;
  }

  .key-badge {
    font-size: 9px;
    font-weight: 600;
    text-transform: uppercase;
    background: var(--color-crush-muted);
    color: var(--color-crush-black);
    padding: 1px 4px;
    border-radius: 3px;
  }

  .key-badge.str {
    background: var(--color-crush-green);
  }

  .key-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .key-workspace {
    padding: 16px;
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .key-card {
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    max-width: 600px;
  }

  .key-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .key-header h2 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
  }

  .key-metadata {
    display: flex;
    gap: 20px;
    font-size: 12px;
    color: var(--color-crush-text-muted);
    align-items: center;
  }

  .ttl-input {
    width: 80px;
    padding: 2px 6px;
  }

  .key-editor {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .key-editor label {
    font-size: 12px;
    font-weight: 500;
  }

  .key-val-textarea {
    height: 200px;
    font-family: var(--font-mono);
    font-size: 13px;
  }

  /* Mongo Layout */
  .mongo-layout {
    display: grid;
    grid-template-columns: 240px 1fr;
    height: 100%;
    overflow: hidden;
  }

  .mongo-sidebar {
    border-right: 1px solid var(--color-crush-border);
    background: var(--color-crush-dark);
    display: flex;
    flex-direction: column;
    padding: 12px;
    gap: 12px;
  }

  .db-switcher {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .db-switcher label {
    font-size: 11px;
    text-transform: uppercase;
    color: var(--color-crush-text-muted);
    font-weight: 500;
  }

  .mongo-workspace {
    padding: 16px;
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow-y: auto;
    gap: 12px;
  }

  .mongo-docs-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .doc-card {
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .doc-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--color-crush-border);
    padding-bottom: 6px;
  }

  .doc-content {
    margin: 0;
    font-family: var(--font-mono);
    font-size: 11px;
    color: #a7a7a7;
    white-space: pre-wrap;
  }

  .doc-editor {
    height: 180px;
    font-family: var(--font-mono);
    font-size: 12px;
  }

  /* Backups tab */
  .backups-layout {
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .backups-layout h2 {
    font-size: 18px;
    font-weight: 600;
    margin: 0;
  }

  /* Modals */
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal-card {
    width: 100%;
    max-width: 500px;
    max-height: 80vh;
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    overflow: hidden;
  }

  .modal-card h3 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
  }

  .modal-fields {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding-right: 4px;
  }

  .field-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .field-row label {
    font-size: 11px;
    font-weight: 500;
    color: var(--color-crush-text-muted);
  }

  .type-hint {
    font-size: 9px;
    font-weight: normal;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    border-top: 1px solid var(--color-crush-border);
    padding-top: 12px;
  }

  /* Common UI Elements */
  .ctable {
    display: flex;
    flex-direction: column;
    font-size: 13px;
    border: 1px solid var(--color-crush-border);
    border-radius: 8px;
    background: var(--color-crush-surface);
    overflow: hidden;
  }

  .crow {
    display: grid;
    grid-template-columns: 2fr 1fr 1fr 1fr;
    align-items: center;
    padding: 10px 16px;
    border-bottom: 1px solid var(--color-crush-border);
    gap: 12px;
  }

  .crow:last-child {
    border-bottom: none;
  }

  .chead {
    background: rgba(0,0,0,0.2);
    font-weight: 500;
    color: var(--color-crush-text-muted);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .bold { font-weight: 600; }
  .mono { font-family: var(--font-mono); }
  .text-xs { font-size: 11px; }

  .ghost-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: none;
    border: 1px solid var(--color-crush-border);
    color: var(--color-crush-text-muted);
    border-radius: 6px;
    padding: 4px 8px;
    cursor: pointer;
  }

  .ghost-btn:hover {
    color: var(--color-crush-text);
    border-color: var(--color-crush-muted);
  }

  .text-red { color: var(--color-crush-red); }
  .text-red:hover {
    border-color: rgba(239, 68, 68, 0.2);
    background: rgba(239, 68, 68, 0.05);
  }

  /* PGMQ Queue Styles */
  .pgmq-layout {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 400px;
  }

  .queues-workspace-layout {
    display: grid;
    grid-template-columns: 240px 1fr;
    gap: 16px;
    height: 100%;
    align-items: start;
  }

  .queues-sidebar {
    background: var(--color-crush-dark);
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-height: 400px;
  }

  .queues-sidebar .sidebar-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--color-crush-border);
    padding-bottom: 8px;
    margin-bottom: 4px;
  }

  .queues-sidebar .sidebar-header h3 {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    text-transform: uppercase;
    color: var(--color-crush-text-muted);
  }

  .queue-data-workspace {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .queue-header-card {
    background: var(--color-crush-dark);
    padding: 16px;
  }

  .queue-info-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .queue-info-row h2 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
  }

  .metrics-grid {
    display: flex;
    gap: 16px;
    flex-wrap: wrap;
  }

  .metric-item {
    display: flex;
    flex-direction: column;
    background: var(--color-crush-surface);
    padding: 6px 12px;
    border-radius: 6px;
    border: 1px solid var(--color-crush-border);
    min-width: 80px;
  }

  .metric-item .lbl {
    font-size: 10px;
    color: var(--color-crush-text-muted);
    text-transform: uppercase;
  }

  .metric-item .val {
    font-size: 13px;
    font-weight: 600;
  }

  .queue-messages-card {
    background: var(--color-crush-dark);
    padding: 16px;
  }

  .msg-row {
    grid-template-columns: 80px 140px 80px 140px 1fr 120px;
  }

  .payload-cell {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 250px;
    color: var(--color-crush-text);
  }

  .delete-queue-icon-btn {
    opacity: 0;
    transition: opacity 0.2s ease;
  }

  .sidebar-item:hover .delete-queue-icon-btn {
    opacity: 1;
  }

  .empty-icon-wrapper {
    display: flex;
    justify-content: center;
    align-items: center;
    height: 96px;
    width: 96px;
    border-radius: 50%;
    background: rgba(var(--color-crush-primary-rgb, 100, 116, 139), 0.1);
    margin: 0 auto;
  }
</style>
