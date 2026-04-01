import { useApi } from './useApi'

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

interface ApiQueryResponse {
  query: string
  rows: Record<string, unknown>[] | null
  message: string | null
  execution_time_ms: number
}

export const useQueryRunner = () => {
  const api = useApi()

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

    const clientStart = Date.now()
    try {
      const data = await api.post<ApiQueryResponse>('/meta/query', { query: sql.trim() })

      executionTime.value = data.execution_time_ms ?? (Date.now() - clientStart)

      if (data.rows) {
        // Build column list from the first row keys (SELECT JSON preserves order)
        const columns = data.rows.length > 0 ? Object.keys(data.rows[0]!) : []
        lastResult.value = { columns, rows: data.rows, rowCount: data.rows.length }
        lastMessage.value = `${data.rows.length} row(s) returned in ${executionTime.value}ms`
        isError.value = false
      } else {
        lastMessage.value = data.message ?? `Query executed in ${executionTime.value}ms`
        isError.value = false
      }

      queryHistory.value.unshift({
        sql,
        result: lastResult.value,
        message: lastMessage.value,
        executionTime: executionTime.value,
        isError: false,
        timestamp: new Date(),
      })
    } catch (e: unknown) {
      const raw = e as { data?: { error?: string }, message?: string }
      const msg = raw?.data?.error ?? raw?.message ?? String(e)
      lastMessage.value = msg
      isError.value = true
      executionTime.value = Date.now() - clientStart

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
