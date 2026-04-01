// ── Types ──────────────────────────────────────────────────────────────────────

import { useApi } from './useApi'

export interface Column {
  name: string
  type: string
  isPartitionKey: boolean
  isClusteringKey: boolean
  isPrimaryKey: boolean
}

export interface Table {
  name: string
  columns: Column[]
  rows: Record<string, unknown>[]
}

export interface Keyspace {
  name: string
  replicationStrategy: string
  replicationFactor: number
  tables: Table[]
}

// ── Raw API shapes ─────────────────────────────────────────────────────────────

interface ApiKeyspace {
  name: string
  replication: Record<string, string>
  durable_writes: boolean
}

interface ApiTable {
  keyspace: string
  name: string
}

interface ApiColumn {
  keyspace: string
  table: string
  name: string
  kind: string   // "partition_key" | "clustering" | "regular" | "static"
  position: number
  data_type: string
}

interface ApiRowsResult {
  keyspace: string
  table: string
  rows: Record<string, unknown>[]
  page_size: number
}

// ── Helpers ────────────────────────────────────────────────────────────────────

/** System keyspaces we hide from the UI */
const HIDDEN_KEYSPACES = new Set(['system', 'system_schema', 'system_auth', 'system_distributed', 'system_traces'])

function mapKeyspace(k: ApiKeyspace): Keyspace {
  const rf = parseInt(k.replication.replication_factor ?? '1', 10)
  const cls = k.replication.class ?? ''
  const strategy = cls.includes('NetworkTopology') ? 'NetworkTopologyStrategy' : 'SimpleStrategy'
  return { name: k.name, replicationStrategy: strategy, replicationFactor: isNaN(rf) ? 1 : rf, tables: [] }
}

function mapColumns(apiCols: ApiColumn[]): Column[] {
  return apiCols
    .sort((a, b) => a.position - b.position)
    .map(c => ({
      name: c.name,
      type: c.data_type,
      isPartitionKey: c.kind === 'partition_key',
      isClusteringKey: c.kind === 'clustering',
      isPrimaryKey: c.kind === 'partition_key' || c.kind === 'clustering',
    }))
}

// ── Composable ─────────────────────────────────────────────────────────────────

export const useDatabase = () => {
  const api = useApi()

  const keyspaces = useState<Keyspace[]>('db_keyspaces', () => [])
  const activeKeyspace = useState<string>('db_activeKeyspace', () => '')
  const activeTable = useState<string | null>('db_activeTable', () => null)

  // Loading / error state exposed for the UI
  const isLoadingKeyspaces = useState<boolean>('db_loadingKS', () => false)
  const isLoadingTable = useState<boolean>('db_loadingTable', () => false)
  const dbError = useState<string | null>('db_error', () => null)

  const currentKeyspace = computed(() => keyspaces.value.find(k => k.name === activeKeyspace.value))
  const currentTables = computed(() => currentKeyspace.value?.tables ?? [])
  const currentTableData = computed(() => {
    if (!activeTable.value) return null
    return currentKeyspace.value?.tables.find(t => t.name === activeTable.value) ?? null
  })

  // ── Fetch all keyspaces (called once on app mount) ──────────────────────────
  async function loadKeyspaces() {
    isLoadingKeyspaces.value = true
    dbError.value = null
    try {
      const data = await api.get<ApiKeyspace[]>('/meta/keyspaces')
      const visible = data.filter((k: ApiKeyspace) => !HIDDEN_KEYSPACES.has(k.name)).map(mapKeyspace)
      keyspaces.value = visible
      const activeKs = activeKeyspace.value && visible.find((k: Keyspace) => k.name === activeKeyspace.value)
        ? activeKeyspace.value
        : visible[0]?.name ?? ''
      activeKeyspace.value = activeKs
      if (activeKs) await loadTables(activeKs)
    } catch (e: unknown) {
      dbError.value = e instanceof Error ? e.message : String(e)
    } finally {
      isLoadingKeyspaces.value = false
    }
  }

  // ── Fetch tables for a keyspace (lazy: only if not yet loaded) ──────────────
  async function loadTables(ksName: string) {
    const ks = keyspaces.value.find(k => k.name === ksName)
    if (!ks) return
    try {
      const data = await api.get<ApiTable[]>(`/meta/keyspaces/${ksName}/tables`)
      // Preserve existing table objects (columns/rows already fetched)
      const existing = new Map(ks.tables.map(t => [t.name, t]))
      ks.tables = data.map(t => existing.get(t.name) ?? { name: t.name, columns: [], rows: [] })
    } catch (e: unknown) {
      dbError.value = e instanceof Error ? e.message : String(e)
    }
  }

  // ── Fetch columns + rows for a specific table ───────────────────────────────
  async function loadTableData(ksName: string, tableName: string) {
    isLoadingTable.value = true
    dbError.value = null
    try {
      const [colData, rowData] = await Promise.all([
        api.get<ApiColumn[]>(`/meta/keyspaces/${ksName}/tables/${tableName}/columns`),
        api.get<ApiRowsResult>(`/meta/keyspaces/${ksName}/tables/${tableName}/rows?page_size=500`),
      ])
      const ks = keyspaces.value.find(k => k.name === ksName)
      if (!ks) return
      const table = ks.tables.find(t => t.name === tableName)
      if (!table) return
      table.columns = mapColumns(colData)
      table.rows = rowData.rows
    } catch (e: unknown) {
      dbError.value = e instanceof Error ? e.message : String(e)
    } finally {
      isLoadingTable.value = false
    }
  }

  // ── Select keyspace ─────────────────────────────────────────────────────────
  async function selectKeyspace(name: string) {
    activeKeyspace.value = name
    activeTable.value = null
    await loadTables(name)
  }

  // ── Select table (fetches data if columns not yet loaded) ───────────────────
  async function selectTable(name: string) {
    activeTable.value = name
    const ks = currentKeyspace.value
    if (!ks) return
    const table = ks.tables.find(t => t.name === name)
    if (!table || table.columns.length === 0) {
      await loadTableData(ks.name, name)
    }
  }

  // ── Create keyspace ─────────────────────────────────────────────────────────
  async function createKeyspace(name: string, replicationFactor: number) {
    await api.post('/meta/keyspaces', {
      name,
      replication: {
        class: 'org.apache.cassandra.locator.SimpleStrategy',
        replication_factor: String(replicationFactor),
      },
    })
    await loadKeyspaces()
    activeKeyspace.value = name
  }

  // ── Create table ────────────────────────────────────────────────────────────
  async function createTable(keyspaceName: string, table: Table) {
    const body = {
      name: table.name,
      columns: table.columns.map(c => ({
        name: c.name,
        data_type: c.type,
        kind: c.isPartitionKey ? 'partition_key' : c.isClusteringKey ? 'clustering' : 'regular',
      })),
    }
    await api.post(`/meta/keyspaces/${keyspaceName}/tables`, body)
    // Add optimistically so UI updates immediately, then reload tables
    await loadTables(keyspaceName)
  }

  // ── Drop table ──────────────────────────────────────────────────────────────
  async function dropTable(keyspaceName: string, tableName: string) {
    await api.del(`/meta/keyspaces/${keyspaceName}/tables/${tableName}`)
    const ks = keyspaces.value.find(k => k.name === keyspaceName)
    if (ks) ks.tables = ks.tables.filter(t => t.name !== tableName)
    if (activeTable.value === tableName) activeTable.value = null
  }

  // ── Insert row ──────────────────────────────────────────────────────────────
  async function insertRow(keyspaceName: string, tableName: string, row: Record<string, unknown>) {
    await api.post(`/meta/keyspaces/${keyspaceName}/tables/${tableName}/rows`, { data: row })
    await loadTableData(keyspaceName, tableName)
  }

  // ── Update row ──────────────────────────────────────────────────────────────
  async function updateRow(
    keyspaceName: string,
    tableName: string,
    set: Record<string, unknown>,
    where: Record<string, unknown>,
  ) {
    await api.patch(`/meta/keyspaces/${keyspaceName}/tables/${tableName}/rows`, { set, where })
    await loadTableData(keyspaceName, tableName)
  }

  // ── Delete row ──────────────────────────────────────────────────────────────
  async function deleteRow(
    keyspaceName: string,
    tableName: string,
    where: Record<string, unknown>,
  ) {
    await api.del(`/meta/keyspaces/${keyspaceName}/tables/${tableName}/rows`, { where })
    await loadTableData(keyspaceName, tableName)
  }

  // ── Helpers ─────────────────────────────────────────────────────────────────
  function getTable(keyspaceName: string, tableName: string): Table | undefined {
    return keyspaces.value.find(k => k.name === keyspaceName)?.tables.find(t => t.name === tableName)
  }

  return {
    // state
    keyspaces,
    activeKeyspace,
    activeTable,
    isLoadingKeyspaces,
    isLoadingTable,
    dbError,
    // computed
    currentKeyspace,
    currentTables,
    currentTableData,
    // actions
    loadKeyspaces,
    loadTables,
    loadTableData,
    selectKeyspace,
    selectTable,
    createKeyspace,
    createTable,
    dropTable,
    insertRow,
    updateRow,
    deleteRow,
    getTable,
  }
}

