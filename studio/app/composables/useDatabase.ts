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

const INITIAL_DATA: Keyspace[] = [
  {
    name: 'ecommerce',
    replicationStrategy: 'SimpleStrategy',
    replicationFactor: 3,
    tables: [
      {
        name: 'users',
        columns: [
          { name: 'user_id', type: 'uuid', isPartitionKey: true, isClusteringKey: false, isPrimaryKey: true },
          { name: 'email', type: 'text', isPartitionKey: false, isClusteringKey: false, isPrimaryKey: false },
          { name: 'username', type: 'text', isPartitionKey: false, isClusteringKey: false, isPrimaryKey: false },
          { name: 'full_name', type: 'text', isPartitionKey: false, isClusteringKey: false, isPrimaryKey: false },
          { name: 'created_at', type: 'timestamp', isPartitionKey: false, isClusteringKey: false, isPrimaryKey: false },
          { name: 'is_active', type: 'boolean', isPartitionKey: false, isClusteringKey: false, isPrimaryKey: false },
        ],
        rows: [
          { user_id: '550e8400-e29b-41d4-a716-446655440001', email: 'alice@example.com', username: 'alice_wonder', full_name: 'Alice Wonderland', created_at: '2024-01-15 10:23:45', is_active: true },
          { user_id: '550e8400-e29b-41d4-a716-446655440002', email: 'bob@example.com', username: 'bob_builder', full_name: 'Bob Builder', created_at: '2024-02-20 14:45:01', is_active: true },
          { user_id: '550e8400-e29b-41d4-a716-446655440003', email: 'carol@example.com', username: 'carol_k', full_name: 'Carol King', created_at: '2024-03-01 09:12:33', is_active: false },
          { user_id: '550e8400-e29b-41d4-a716-446655440004', email: 'dave@example.com', username: 'dave99', full_name: 'Dave Smith', created_at: '2024-03-10 16:08:22', is_active: true },
          { user_id: '550e8400-e29b-41d4-a716-446655440005', email: 'eve@example.com', username: 'eve_online', full_name: 'Eve Adams', created_at: '2024-04-05 11:55:10', is_active: true },
          { user_id: '550e8400-e29b-41d4-a716-446655440006', email: 'frank@example.com', username: 'frank_sinatra', full_name: 'Frank Sinatra', created_at: '2024-04-12 08:30:00', is_active: false },
        ],
      },
      {
        name: 'products',
        columns: [
          { name: 'product_id', type: 'uuid', isPartitionKey: true, isClusteringKey: false, isPrimaryKey: true },
          { name: 'category', type: 'text', isPartitionKey: false, isClusteringKey: true, isPrimaryKey: true },
          { name: 'name', type: 'text', isPartitionKey: false, isClusteringKey: false, isPrimaryKey: false },
          { name: 'price', type: 'decimal', isPartitionKey: false, isClusteringKey: false, isPrimaryKey: false },
          { name: 'stock', type: 'int', isPartitionKey: false, isClusteringKey: false, isPrimaryKey: false },
          { name: 'description', type: 'text', isPartitionKey: false, isClusteringKey: false, isPrimaryKey: false },
        ],
        rows: [
          { product_id: 'a72e8400-e29b-41d4-a716-446655440001', category: 'Electronics', name: 'Wireless Headphones', price: 79.99, stock: 150, description: 'High-quality wireless headphones with 40h battery' },
          { product_id: 'a72e8400-e29b-41d4-a716-446655440002', category: 'Electronics', name: 'Mechanical Keyboard', price: 129.00, stock: 85, description: 'RGB mechanical keyboard, Cherry MX switches' },
          { product_id: 'a72e8400-e29b-41d4-a716-446655440003', category: 'Clothing', name: 'Running Shoes', price: 99.95, stock: 200, description: 'Lightweight breathable running shoes' },
          { product_id: 'a72e8400-e29b-41d4-a716-446655440004', category: 'Books', name: 'Clean Code', price: 34.50, stock: 45, description: 'A handbook of agile software craftsmanship' },
          { product_id: 'a72e8400-e29b-41d4-a716-446655440005', category: 'Electronics', name: '4K Monitor', price: 449.99, stock: 30, description: '27" 4K IPS monitor 144Hz' },
        ],
      },
      {
        name: 'orders',
        columns: [
          { name: 'order_id', type: 'uuid', isPartitionKey: true, isClusteringKey: false, isPrimaryKey: true },
          { name: 'user_id', type: 'uuid', isPartitionKey: false, isClusteringKey: false, isPrimaryKey: false },
          { name: 'total', type: 'decimal', isPartitionKey: false, isClusteringKey: false, isPrimaryKey: false },
          { name: 'status', type: 'text', isPartitionKey: false, isClusteringKey: false, isPrimaryKey: false },
          { name: 'created_at', type: 'timestamp', isPartitionKey: false, isClusteringKey: false, isPrimaryKey: false },
        ],
        rows: [
          { order_id: 'b82e8400-e29b-41d4-a716-446655440001', user_id: '550e8400-e29b-41d4-a716-446655440001', total: 209.99, status: 'delivered', created_at: '2024-05-01 10:00:00' },
          { order_id: 'b82e8400-e29b-41d4-a716-446655440002', user_id: '550e8400-e29b-41d4-a716-446655440002', total: 34.50, status: 'shipped', created_at: '2024-05-05 14:30:00' },
          { order_id: 'b82e8400-e29b-41d4-a716-446655440003', user_id: '550e8400-e29b-41d4-a716-446655440001', total: 99.95, status: 'pending', created_at: '2024-05-10 09:15:00' },
          { order_id: 'b82e8400-e29b-41d4-a716-446655440004', user_id: '550e8400-e29b-41d4-a716-446655440003', total: 579.99, status: 'processing', created_at: '2024-05-12 17:00:00' },
        ],
      },
      {
        name: 'sessions',
        columns: [
          { name: 'session_id', type: 'uuid', isPartitionKey: true, isClusteringKey: false, isPrimaryKey: true },
          { name: 'user_id', type: 'uuid', isPartitionKey: false, isClusteringKey: false, isPrimaryKey: false },
          { name: 'ip_address', type: 'inet', isPartitionKey: false, isClusteringKey: false, isPrimaryKey: false },
          { name: 'user_agent', type: 'text', isPartitionKey: false, isClusteringKey: false, isPrimaryKey: false },
          { name: 'expires_at', type: 'timestamp', isPartitionKey: false, isClusteringKey: false, isPrimaryKey: false },
        ],
        rows: [
          { session_id: 'c92e8400-e29b-41d4-a716-446655440001', user_id: '550e8400-e29b-41d4-a716-446655440001', ip_address: '192.168.1.10', user_agent: 'Mozilla/5.0 Chrome/120', expires_at: '2024-05-13 10:00:00' },
          { session_id: 'c92e8400-e29b-41d4-a716-446655440002', user_id: '550e8400-e29b-41d4-a716-446655440002', ip_address: '10.0.0.55', user_agent: 'Mozilla/5.0 Firefox/121', expires_at: '2024-05-14 08:00:00' },
        ],
      },
    ],
  },
  {
    name: 'analytics',
    replicationStrategy: 'NetworkTopologyStrategy',
    replicationFactor: 2,
    tables: [
      {
        name: 'page_views',
        columns: [
          { name: 'date', type: 'date', isPartitionKey: true, isClusteringKey: false, isPrimaryKey: true },
          { name: 'page', type: 'text', isPartitionKey: false, isClusteringKey: true, isPrimaryKey: true },
          { name: 'views', type: 'counter', isPartitionKey: false, isClusteringKey: false, isPrimaryKey: false },
          { name: 'unique_visitors', type: 'counter', isPartitionKey: false, isClusteringKey: false, isPrimaryKey: false },
        ],
        rows: [
          { date: '2024-05-01', page: '/home', views: 12450, unique_visitors: 8320 },
          { date: '2024-05-01', page: '/products', views: 7830, unique_visitors: 5410 },
          { date: '2024-05-02', page: '/home', views: 11200, unique_visitors: 7900 },
          { date: '2024-05-02', page: '/checkout', views: 3200, unique_visitors: 2800 },
        ],
      },
      {
        name: 'events',
        columns: [
          { name: 'event_id', type: 'uuid', isPartitionKey: true, isClusteringKey: false, isPrimaryKey: true },
          { name: 'event_type', type: 'text', isPartitionKey: false, isClusteringKey: false, isPrimaryKey: false },
          { name: 'user_id', type: 'uuid', isPartitionKey: false, isClusteringKey: false, isPrimaryKey: false },
          { name: 'properties', type: 'map<text, text>', isPartitionKey: false, isClusteringKey: false, isPrimaryKey: false },
          { name: 'timestamp', type: 'timestamp', isPartitionKey: false, isClusteringKey: false, isPrimaryKey: false },
        ],
        rows: [
          { event_id: 'd02e8400-e29b-41d4-a716-446655440001', event_type: 'page_view', user_id: '550e8400-e29b-41d4-a716-446655440001', properties: '{page: /home}', timestamp: '2024-05-10 10:00:01' },
          { event_id: 'd02e8400-e29b-41d4-a716-446655440002', event_type: 'click', user_id: '550e8400-e29b-41d4-a716-446655440002', properties: '{element: buy-btn}', timestamp: '2024-05-10 10:05:22' },
        ],
      },
    ],
  },
  {
    name: 'system',
    replicationStrategy: 'LocalStrategy',
    replicationFactor: 1,
    tables: [
      {
        name: 'local',
        columns: [
          { name: 'key', type: 'text', isPartitionKey: true, isClusteringKey: false, isPrimaryKey: true },
          { name: 'cluster_name', type: 'text', isPartitionKey: false, isClusteringKey: false, isPrimaryKey: false },
          { name: 'release_version', type: 'text', isPartitionKey: false, isClusteringKey: false, isPrimaryKey: false },
          { name: 'cql_version', type: 'text', isPartitionKey: false, isClusteringKey: false, isPrimaryKey: false },
          { name: 'data_center', type: 'text', isPartitionKey: false, isClusteringKey: false, isPrimaryKey: false },
        ],
        rows: [
          { key: 'local', cluster_name: 'My ScyllaDB Cluster', release_version: '5.4.0', cql_version: '3.3.1', data_center: 'datacenter1' },
        ],
      },
    ],
  },
]

export const useDatabase = () => {
  const keyspaces = useState<Keyspace[]>('db_keyspaces', () => INITIAL_DATA)
  const activeKeyspace = useState<string>('db_activeKeyspace', () => 'ecommerce')
  const activeTable = useState<string | null>('db_activeTable', () => null)

  const currentKeyspace = computed(() => keyspaces.value.find(k => k.name === activeKeyspace.value))
  const currentTables = computed(() => currentKeyspace.value?.tables ?? [])
  const currentTableData = computed(() => {
    if (!activeTable.value) return null
    return currentKeyspace.value?.tables.find(t => t.name === activeTable.value) ?? null
  })

  function selectKeyspace(name: string) {
    activeKeyspace.value = name
    activeTable.value = null
  }

  function selectTable(name: string) {
    activeTable.value = name
  }

  function createTable(keyspaceName: string, table: Table) {
    const ks = keyspaces.value.find(k => k.name === keyspaceName)
    if (!ks) throw new Error(`Keyspace '${keyspaceName}' not found`)
    if (ks.tables.find(t => t.name === table.name)) throw new Error(`Table '${table.name}' already exists`)
    ks.tables.push(table)
  }

  function dropTable(keyspaceName: string, tableName: string) {
    const ks = keyspaces.value.find(k => k.name === keyspaceName)
    if (ks) {
      ks.tables = ks.tables.filter(t => t.name !== tableName)
      if (activeTable.value === tableName) activeTable.value = null
    }
  }

  function insertRow(keyspaceName: string, tableName: string, row: Record<string, unknown>) {
    const ks = keyspaces.value.find(k => k.name === keyspaceName)
    const table = ks?.tables.find(t => t.name === tableName)
    if (table) table.rows.push(row)
  }

  function getTable(keyspaceName: string, tableName: string): Table | undefined {
    return keyspaces.value.find(k => k.name === keyspaceName)?.tables.find(t => t.name === tableName)
  }

  return {
    keyspaces,
    activeKeyspace,
    activeTable,
    currentKeyspace,
    currentTables,
    currentTableData,
    selectKeyspace,
    selectTable,
    createTable,
    dropTable,
    insertRow,
    getTable,
  }
}
