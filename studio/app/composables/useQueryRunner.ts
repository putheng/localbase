export interface QueryResult {
  columns: string[]
  rows: Record<string, unknown>[]
  rowCount: number
}

export interface QueryExecution {
  sql: string
  result: QueryResult | null
  message: string
  executionTime: number
  isError: boolean
  timestamp: Date
}

export const useQueryRunner = () => {
  const { keyspaces, activeKeyspace, createTable, dropTable } = useDatabase()

  const queryHistory = useState<QueryExecution[]>('query_history', () => [])
  const lastResult = useState<QueryResult | null>('query_lastResult', () => null)
  const lastMessage = useState<string>('query_lastMessage', () => '')
  const isRunning = useState<boolean>('query_isRunning', () => false)
  const isError = useState<boolean>('query_isError', () => false)
  const executionTime = useState<number>('query_execTime', () => 0)

  async function runQuery(sql: string) {
    if (!sql.trim()) return
    isRunning.value = true
    lastResult.value = null
    lastMessage.value = ''
    isError.value = false

    const start = Date.now()
    await new Promise(resolve => setTimeout(resolve, 150 + Math.random() * 250))

    try {
      const result = parseAndExecute(sql.trim())
      executionTime.value = Date.now() - start

      if (result.type === 'select') {
        lastResult.value = result.data
        lastMessage.value = `${result.data.rowCount} row(s) returned in ${executionTime.value}ms`
        isError.value = false
      } else {
        lastMessage.value = result.message
        isError.value = false
      }

      queryHistory.value.unshift({
        sql,
        result: result.type === 'select' ? result.data : null,
        message: lastMessage.value,
        executionTime: executionTime.value,
        isError: false,
        timestamp: new Date(),
      })
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e)
      lastMessage.value = msg
      isError.value = true
      executionTime.value = Date.now() - start

      queryHistory.value.unshift({
        sql,
        result: null,
        message: msg,
        executionTime: executionTime.value,
        isError: true,
        timestamp: new Date(),
      })
    } finally {
      isRunning.value = false
    }
  }

  function parseAndExecute(sql: string): { type: 'select', data: QueryResult } | { type: 'dml', message: string } {
    const upper = sql.toUpperCase().trimStart()

    if (upper.startsWith('SELECT')) return executeSelect(sql)
    if (upper.startsWith('CREATE TABLE')) return executeCreateTable(sql)
    if (upper.startsWith('DROP TABLE')) return executeDropTable(sql)
    if (upper.startsWith('INSERT INTO')) return { type: 'dml', message: 'INSERT executed. 1 row affected.' }
    if (upper.startsWith('UPDATE')) return { type: 'dml', message: 'UPDATE executed.' }
    if (upper.startsWith('DELETE')) return { type: 'dml', message: 'DELETE executed.' }
    if (upper.startsWith('TRUNCATE')) return { type: 'dml', message: 'Table truncated.' }
    if (upper.startsWith('DESCRIBE') || upper.startsWith('DESC')) return executeDescribe(sql)
    if (upper.startsWith('SHOW')) return executeShow(sql)
    if (upper.startsWith('USE')) return executeUse(sql)

    throw new Error(`Unknown statement. Supported: SELECT, INSERT, UPDATE, DELETE, CREATE TABLE, DROP TABLE, DESCRIBE, SHOW, USE`)
  }

  function resolveTable(rawKs: string | undefined, rawTable: string) {
    const ksName = rawKs || activeKeyspace.value
    const ks = keyspaces.value.find(k => k.name === ksName.toLowerCase())
    if (!ks) throw new Error(`Keyspace '${ksName}' does not exist`)
    const table = ks.tables.find(t => t.name === rawTable.toLowerCase())
    if (!table) throw new Error(`Table '${rawTable}' does not exist in keyspace '${ksName}'`)
    return { ks, table }
  }

  function executeSelect(sql: string): { type: 'select', data: QueryResult } {
    const fromMatch = sql.match(/\bFROM\s+(?:(\w+)\.)?(\w+)/i)
    if (!fromMatch?.[2]) throw new Error('Invalid SELECT: missing table name in FROM clause')

    const { table } = resolveTable(fromMatch[1], fromMatch[2])

    const colMatch = sql.match(/SELECT\s+(.*?)\s+FROM/i)
    const colStr = colMatch?.[1]?.trim() ?? '*'

    let columns: string[]
    let rows: Record<string, unknown>[]

    if (colStr === '*') {
      columns = table.columns.map(c => c.name)
      rows = table.rows.map(r => ({ ...r }))
    } else {
      columns = colStr.split(',').map(c => c.trim().split(/\s+as\s+/i)[0]!.trim())
      rows = table.rows.map(row => {
        const filtered: Record<string, unknown> = {}
        columns.forEach(col => { filtered[col] = (row as Record<string, unknown>)[col] ?? null })
        return filtered
      })
    }

    // WHERE (simple equality only)
    const whereMatch = sql.match(/\bWHERE\s+(\w+)\s*=\s*'?([^'\s;]+)'?/i)
    if (whereMatch) {
      const [, col, val] = whereMatch
      rows = rows.filter(r => String(r[col!]) === String(val))
    }

    // LIMIT
    const limitMatch = sql.match(/\bLIMIT\s+(\d+)/i)
    if (limitMatch) rows = rows.slice(0, parseInt(limitMatch[1]!))

    return { type: 'select', data: { columns, rows, rowCount: rows.length } }
  }

  function executeCreateTable(sql: string): { type: 'dml', message: string } {
    const nameMatch = sql.match(/CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?(?:(\w+)\.)?(\w+)/i)
    if (!nameMatch) throw new Error('Invalid CREATE TABLE syntax')

    const ksName = nameMatch[1] || activeKeyspace.value
    const tableName = nameMatch[2]!.toLowerCase()

    createTable(ksName, {
      name: tableName,
      columns: [
        { name: 'id', type: 'uuid', isPartitionKey: true, isClusteringKey: false, isPrimaryKey: true },
        { name: 'created_at', type: 'timestamp', isPartitionKey: false, isClusteringKey: false, isPrimaryKey: false },
      ],
      rows: [],
    })

    return { type: 'dml', message: `Table '${ksName}.${tableName}' created successfully.` }
  }

  function executeDropTable(sql: string): { type: 'dml', message: string } {
    const nameMatch = sql.match(/DROP\s+TABLE\s+(?:IF\s+EXISTS\s+)?(?:(\w+)\.)?(\w+)/i)
    if (!nameMatch) throw new Error('Invalid DROP TABLE syntax')

    const ksName = nameMatch[1] || activeKeyspace.value
    const tableName = nameMatch[2]!.toLowerCase()

    dropTable(ksName, tableName)
    return { type: 'dml', message: `Table '${ksName}.${tableName}' dropped.` }
  }

  function executeDescribe(sql: string): { type: 'select', data: QueryResult } {
    const nameMatch = sql.match(/(?:DESCRIBE|DESC)\s+(?:TABLE\s+)?(?:(\w+)\.)?(\w+)/i)
    if (!nameMatch) throw new Error('Invalid DESCRIBE syntax. Use: DESCRIBE [keyspace.]table')

    const { table } = resolveTable(nameMatch[1], nameMatch[2]!)

    const columns = ['column_name', 'type', 'partition_key', 'clustering_key']
    const rows = table.columns.map(col => ({
      column_name: col.name,
      type: col.type,
      partition_key: col.isPartitionKey ? 'YES' : 'NO',
      clustering_key: col.isClusteringKey ? 'YES' : 'NO',
    }))

    return { type: 'select', data: { columns, rows, rowCount: rows.length } }
  }

  function executeShow(sql: string): { type: 'select', data: QueryResult } {
    const upper = sql.toUpperCase()
    if (upper.includes('KEYSPACE') || upper.includes('KEYSPACES')) {
      const rows = keyspaces.value.map(ks => ({
        keyspace_name: ks.name,
        replication_strategy: ks.replicationStrategy,
        replication_factor: ks.replicationFactor,
        tables: ks.tables.length,
      }))
      return { type: 'select', data: { columns: ['keyspace_name', 'replication_strategy', 'replication_factor', 'tables'], rows, rowCount: rows.length } }
    }
    if (upper.includes('TABLES')) {
      const ks = keyspaces.value.find(k => k.name === activeKeyspace.value)
      const rows = (ks?.tables ?? []).map(t => ({
        table_name: t.name,
        columns: t.columns.length,
        rows: t.rows.length,
      }))
      return { type: 'select', data: { columns: ['table_name', 'columns', 'rows'], rows, rowCount: rows.length } }
    }
    throw new Error('Unknown SHOW statement. Try: SHOW KEYSPACES or SHOW TABLES')
  }

  function executeUse(sql: string): { type: 'dml', message: string } {
    const match = sql.match(/USE\s+(\w+)/i)
    if (!match) throw new Error('Invalid USE syntax')
    const ksName = match[1]!.toLowerCase()
    const exists = keyspaces.value.find(k => k.name === ksName)
    if (!exists) throw new Error(`Keyspace '${ksName}' does not exist`)
    activeKeyspace.value = ksName
    return { type: 'dml', message: `Switched to keyspace '${ksName}'` }
  }

  return {
    queryHistory,
    lastResult,
    lastMessage,
    isRunning,
    isError,
    executionTime,
    runQuery,
  }
}
