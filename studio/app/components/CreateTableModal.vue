<script setup lang="ts">
import type { Column } from '~/composables/useDatabase'

const props = defineProps<{ keyspaceName: string }>()
const emit = defineEmits<{ close: []; created: [tableName: string] }>()

const { createTable } = useDatabase()

const tableName = ref('')
const columns = ref<Array<{ name: string; type: string; isPartitionKey: boolean; isClusteringKey: boolean }>>([
  { name: 'id', type: 'uuid', isPartitionKey: true, isClusteringKey: false },
  { name: 'created_at', type: 'timestamp', isPartitionKey: false, isClusteringKey: false },
])

const COLUMN_TYPES = [
  'uuid', 'text', 'varchar', 'int', 'bigint', 'smallint', 'tinyint',
  'float', 'double', 'decimal', 'boolean', 'timestamp', 'date', 'time',
  'blob', 'inet', 'counter', 'duration', 'list<text>', 'set<text>', 'map<text, text>',
]

const error = ref('')

function addColumn() {
  columns.value.push({ name: '', type: 'text', isPartitionKey: false, isClusteringKey: false })
}

function removeColumn(index: number) {
  if (columns.value.length <= 1) return
  columns.value.splice(index, 1)
}

function handleCreate() {
  error.value = ''

  if (!tableName.value.trim()) {
    error.value = 'Table name is required'
    return
  }
  if (!/^[a-z_][a-z0-9_]*$/i.test(tableName.value)) {
    error.value = 'Table name must contain only letters, numbers, and underscores'
    return
  }

  const partitionKeys = columns.value.filter(c => c.isPartitionKey)
  if (partitionKeys.length === 0) {
    error.value = 'At least one partition key is required'
    return
  }

  for (const col of columns.value) {
    if (!col.name.trim()) {
      error.value = 'All columns must have a name'
      return
    }
  }

  const names = columns.value.map(c => c.name)
  if (new Set(names).size !== names.length) {
    error.value = 'Column names must be unique'
    return
  }

  try {
    createTable(props.keyspaceName, {
      name: tableName.value.trim().toLowerCase(),
      columns: columns.value.map(c => ({
        name: c.name.trim().toLowerCase(),
        type: c.type,
        isPartitionKey: c.isPartitionKey,
        isClusteringKey: c.isClusteringKey,
        isPrimaryKey: c.isPartitionKey || c.isClusteringKey,
      } satisfies Column)),
      rows: [],
    })
    emit('created', tableName.value.trim().toLowerCase())
    emit('close')
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : 'Failed to create table'
  }
}
</script>

<template>
  <!-- Backdrop -->
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm" @click.self="emit('close')">
    <div class="bg-slate-900 rounded-xl border border-slate-700 w-full max-w-2xl max-h-[90vh] flex flex-col shadow-2xl">
      <!-- Header -->
      <div class="flex items-center justify-between px-6 py-4 border-b border-slate-800 shrink-0">
        <div>
          <h2 class="text-base font-semibold text-slate-100">Create New Table</h2>
          <p class="text-xs text-slate-500 mt-0.5">Keyspace: <span class="text-violet-400">{{ keyspaceName }}</span></p>
        </div>
        <button @click="emit('close')" class="text-slate-500 hover:text-slate-300 transition-colors p-1 rounded">
          <svg class="size-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
          </svg>
        </button>
      </div>

      <!-- Body -->
      <div class="flex-1 overflow-y-auto px-6 py-5 flex flex-col gap-5">
        <!-- Table name -->
        <div>
          <label class="block text-xs font-medium text-slate-400 mb-1.5">Table Name</label>
          <input
            v-model="tableName"
            type="text"
            placeholder="e.g. user_events"
            class="w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-100 placeholder-slate-600 focus:outline-none focus:ring-1 focus:ring-violet-500 focus:border-violet-500"
          />
        </div>

        <!-- Columns -->
        <div>
          <div class="flex items-center justify-between mb-2">
            <label class="text-xs font-medium text-slate-400">Columns</label>
            <div class="flex items-center gap-3 text-[10px] text-slate-600">
              <span class="flex items-center gap-1"><span class="size-1.5 rounded-full bg-amber-400 inline-block"></span> Partition Key</span>
              <span class="flex items-center gap-1"><span class="size-1.5 rounded-full bg-blue-400 inline-block"></span> Clustering Key</span>
            </div>
          </div>

          <!-- Column header -->
          <div class="grid grid-cols-[1fr_140px_auto_auto_auto] gap-2 px-2 mb-1">
            <span class="text-[10px] text-slate-600 uppercase tracking-wider">Name</span>
            <span class="text-[10px] text-slate-600 uppercase tracking-wider">Type</span>
            <span class="text-[10px] text-slate-600 uppercase tracking-wider w-16 text-center">Partition</span>
            <span class="text-[10px] text-slate-600 uppercase tracking-wider w-16 text-center">Cluster</span>
            <span class="w-6"></span>
          </div>

          <div class="flex flex-col gap-1.5">
            <div
              v-for="(col, i) in columns"
              :key="i"
              class="grid grid-cols-[1fr_140px_auto_auto_auto] gap-2 items-center"
            >
              <input
                v-model="col.name"
                type="text"
                placeholder="column_name"
                class="bg-slate-800 border border-slate-700 rounded-md px-2.5 py-1.5 text-sm text-slate-100 placeholder-slate-600 focus:outline-none focus:ring-1 focus:ring-violet-500 focus:border-violet-500"
              />
              <select
                v-model="col.type"
                class="bg-slate-800 border border-slate-700 rounded-md px-2 py-1.5 text-sm text-slate-100 focus:outline-none focus:ring-1 focus:ring-violet-500 focus:border-violet-500"
              >
                <option v-for="t in COLUMN_TYPES" :key="t" :value="t">{{ t }}</option>
              </select>
              <div class="flex justify-center w-16">
                <button
                  @click="col.isPartitionKey = !col.isPartitionKey; if(col.isPartitionKey) col.isClusteringKey = false"
                  class="size-5 rounded flex items-center justify-center border transition-colors"
                  :class="col.isPartitionKey ? 'bg-amber-400/20 border-amber-400 text-amber-400' : 'border-slate-700 text-slate-600 hover:border-slate-500'"
                >
                  <svg class="size-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                    <polyline v-if="col.isPartitionKey" points="20 6 9 17 4 12"/>
                  </svg>
                </button>
              </div>
              <div class="flex justify-center w-16">
                <button
                  @click="col.isClusteringKey = !col.isClusteringKey; if(col.isClusteringKey) col.isPartitionKey = false"
                  class="size-5 rounded flex items-center justify-center border transition-colors"
                  :class="col.isClusteringKey ? 'bg-blue-400/20 border-blue-400 text-blue-400' : 'border-slate-700 text-slate-600 hover:border-slate-500'"
                >
                  <svg class="size-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                    <polyline v-if="col.isClusteringKey" points="20 6 9 17 4 12"/>
                  </svg>
                </button>
              </div>
              <button
                @click="removeColumn(i)"
                :disabled="columns.length <= 1"
                class="size-6 flex items-center justify-center text-slate-600 hover:text-rose-400 disabled:opacity-20 disabled:cursor-not-allowed transition-colors rounded"
              >
                <svg class="size-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
                </svg>
              </button>
            </div>
          </div>

          <button
            @click="addColumn"
            class="mt-3 flex items-center gap-1.5 text-xs text-violet-400 hover:text-violet-300 transition-colors"
          >
            <svg class="size-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
              <line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>
            </svg>
            Add Column
          </button>
        </div>

        <!-- Error -->
        <div v-if="error" class="flex items-center gap-2 bg-rose-950/50 border border-rose-800 rounded-lg px-3 py-2.5">
          <svg class="size-4 text-rose-400 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/>
          </svg>
          <span class="text-sm text-rose-300">{{ error }}</span>
        </div>
      </div>

      <!-- Footer -->
      <div class="flex items-center justify-end gap-2.5 px-6 py-4 border-t border-slate-800 shrink-0">
        <button
          @click="emit('close')"
          class="px-4 py-2 text-sm text-slate-400 hover:text-slate-200 transition-colors rounded-lg hover:bg-slate-800"
        >
          Cancel
        </button>
        <button
          @click="handleCreate"
          class="px-4 py-2 text-sm font-medium bg-violet-600 hover:bg-violet-500 text-white rounded-lg transition-colors"
        >
          Create Table
        </button>
      </div>
    </div>
  </div>
</template>
