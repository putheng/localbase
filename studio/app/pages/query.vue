<script setup lang="ts">
const { activeKeyspace } = useDatabase()
const { queryHistory, lastResult, lastMessage, isRunning, isError, executionTime, runQuery } = useQueryRunner()

const queryText = ref(`-- Welcome to ScyllaDB Studio SQL Editor
-- Press Ctrl+Enter (or ⌘+Enter) to run

SELECT * FROM ecommerce.users LIMIT 10;`)

const activeTab = ref<'results' | 'history'>('results')
const panelHeight = ref(45) // percentage for editor panel
const isDragging = ref(false)
const containerRef = ref<HTMLElement>()

async function handleRun() {
  const lines = queryText.value.split('\n').filter(l => l.trim() && !l.trim().startsWith('--'))
  const statement = lines.join('\n').replace(/;.*$/, '').trim()
  if (!statement) return
  await runQuery(statement)
  activeTab.value = 'results'
}

function applyHistory(sql: string) {
  queryText.value = sql
  activeTab.value = 'results'
}

// Resizable panels
function startDrag(e: MouseEvent) {
  isDragging.value = true
  e.preventDefault()
}

function onMouseMove(e: MouseEvent) {
  if (!isDragging.value || !containerRef.value) return
  const rect = containerRef.value.getBoundingClientRect()
  const pct = ((e.clientY - rect.top) / rect.height) * 100
  panelHeight.value = Math.min(75, Math.max(20, pct))
}

function stopDrag() {
  isDragging.value = false
}

const SAMPLE_QUERIES = [
  { label: 'All users', sql: 'SELECT * FROM ecommerce.users LIMIT 10' },
  { label: 'Active users', sql: "SELECT user_id, email, username FROM ecommerce.users WHERE is_active = 'true' LIMIT 10" },
  { label: 'Products', sql: 'SELECT * FROM ecommerce.products' },
  { label: 'Orders', sql: 'SELECT * FROM ecommerce.orders LIMIT 20' },
  { label: 'Show keyspaces', sql: 'SHOW KEYSPACES' },
  { label: 'Show tables', sql: 'SHOW TABLES' },
  { label: 'Describe users', sql: 'DESCRIBE ecommerce.users' },
]
</script>

<template>
  <div
    ref="containerRef"
    class="flex flex-col h-full overflow-hidden select-none"
    @mousemove="onMouseMove"
    @mouseup="stopDrag"
    @mouseleave="stopDrag"
  >
    <!-- Top toolbar -->
    <header class="flex items-center gap-3 px-4 py-2.5 border-b border-slate-800 bg-slate-900/30 shrink-0">
      <div class="flex items-center gap-2">
        <button
          @click="handleRun"
          :disabled="isRunning"
          class="flex items-center gap-2 px-3.5 py-1.5 bg-violet-600 hover:bg-violet-500 disabled:opacity-60 disabled:cursor-not-allowed text-white text-sm font-medium rounded-md transition-colors"
        >
          <svg v-if="!isRunning" class="size-3.5" viewBox="0 0 24 24" fill="currentColor">
            <polygon points="5 3 19 12 5 21 5 3"/>
          </svg>
          <svg v-else class="size-3.5 animate-spin" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"/>
          </svg>
          {{ isRunning ? 'Running…' : 'Run' }}
        </button>
        <span class="text-[10px] text-slate-600 hidden sm:block">Ctrl+Enter</span>
      </div>

      <!-- Sample queries -->
      <div class="flex-1 flex items-center gap-1.5 overflow-x-auto">
        <button
          v-for="sq in SAMPLE_QUERIES"
          :key="sq.label"
          @click="queryText = sq.sql"
          class="shrink-0 text-[11px] px-2 py-1 rounded border border-slate-700/60 text-slate-500 hover:text-slate-300 hover:border-slate-600 hover:bg-slate-800/50 transition-colors whitespace-nowrap"
        >
          {{ sq.label }}
        </button>
      </div>

      <!-- Keyspace badge -->
      <div class="flex items-center gap-1.5 shrink-0 ml-auto">
        <span class="text-xs text-slate-600">Keyspace:</span>
        <span class="text-xs text-violet-400 font-medium bg-violet-500/10 px-2 py-0.5 rounded">{{ activeKeyspace }}</span>
      </div>
    </header>

    <!-- Editor panel -->
    <div
      class="shrink-0 overflow-hidden bg-slate-950"
      :style="{ height: panelHeight + '%' }"
    >
      <ClientOnly>
        <SqlEditor
          v-model="queryText"
          @run="handleRun"
          class="h-full"
        />
        <template #fallback>
          <div class="h-full bg-slate-950 flex items-center justify-center">
            <span class="text-xs text-slate-600">Loading editor…</span>
          </div>
        </template>
      </ClientOnly>
    </div>

    <!-- Drag handle -->
    <div
      class="h-1.5 shrink-0 cursor-row-resize group flex items-center justify-center bg-slate-900 hover:bg-violet-600/20 border-y border-slate-800 transition-colors"
      :class="isDragging ? 'bg-violet-600/20' : ''"
      @mousedown="startDrag"
    >
      <div class="flex gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity">
        <span class="w-6 h-0.5 rounded-full bg-violet-500/60"></span>
      </div>
    </div>

    <!-- Results panel -->
    <div class="flex-1 flex flex-col min-h-0 overflow-hidden">
      <!-- Tabs -->
      <div class="flex items-center gap-0 border-b border-slate-800 px-4 bg-slate-900/20 shrink-0">
        <button
          v-for="tab in ['results', 'history'] as const"
          :key="tab"
          @click="activeTab = tab"
          class="px-3 py-2.5 text-xs font-medium capitalize border-b-2 -mb-px transition-colors"
          :class="activeTab === tab
            ? 'text-violet-300 border-violet-500'
            : 'text-slate-500 border-transparent hover:text-slate-300'"
        >
          {{ tab }}
          <span v-if="tab === 'results' && lastResult" class="ml-1.5 text-[10px] text-slate-600">
            {{ lastResult.rowCount }} rows
          </span>
          <span v-if="tab === 'history' && queryHistory.length" class="ml-1.5 text-[10px] text-slate-600">
            {{ queryHistory.length }}
          </span>
        </button>

        <!-- Execution info -->
        <div v-if="executionTime && activeTab === 'results'" class="ml-auto flex items-center gap-1.5 text-[11px] text-slate-600">
          <svg class="size-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/>
          </svg>
          {{ executionTime }}ms
        </div>
      </div>

      <!-- Results content -->
      <div class="flex-1 overflow-hidden">
        <!-- Results tab -->
        <template v-if="activeTab === 'results'">
          <!-- Status message -->
          <div
            v-if="lastMessage && !lastResult"
            class="flex items-start gap-3 px-5 py-4"
          >
            <div
              class="size-8 rounded-full flex items-center justify-center shrink-0"
              :class="isError ? 'bg-rose-950/50 border border-rose-800' : 'bg-emerald-950/50 border border-emerald-800'"
            >
              <svg v-if="!isError" class="size-4 text-emerald-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <polyline points="20 6 9 17 4 12"/>
              </svg>
              <svg v-else class="size-4 text-rose-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/>
              </svg>
            </div>
            <div>
              <p class="text-sm font-medium" :class="isError ? 'text-rose-300' : 'text-emerald-300'">
                {{ isError ? 'Error' : 'Success' }}
              </p>
              <p class="text-xs text-slate-400 mt-0.5 font-mono">{{ lastMessage }}</p>
            </div>
          </div>

          <!-- Results grid -->
          <div v-else-if="lastResult" class="h-full overflow-auto">
            <table class="w-full border-collapse text-xs">
              <thead class="sticky top-0 bg-slate-900 z-10">
                <tr>
                  <th
                    v-for="col in lastResult.columns"
                    :key="col"
                    class="px-4 py-2.5 text-left border-b border-r border-slate-800 text-slate-400 font-medium whitespace-nowrap"
                  >
                    {{ col }}
                  </th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="(row, ri) in lastResult.rows"
                  :key="ri"
                  class="border-b border-slate-800/50 hover:bg-slate-800/30 transition-colors"
                >
                  <td
                    v-for="col in lastResult.columns"
                    :key="col"
                    class="px-4 py-2 border-r border-slate-800/40 font-mono text-slate-300 max-w-xs truncate"
                    :title="String(row[col] ?? '')"
                  >
                    <span v-if="row[col] === null || row[col] === undefined" class="text-slate-700 italic">null</span>
                    <span v-else>{{ row[col] }}</span>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <!-- Empty state -->
          <div v-else class="flex flex-col items-center justify-center h-full text-center p-8">
            <svg class="size-8 text-slate-700 mb-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/>
            </svg>
            <p class="text-sm text-slate-600">Run a query to see results</p>
            <p class="text-xs text-slate-700 mt-1">Press <kbd class="bg-slate-800 text-slate-500 px-1.5 py-0.5 rounded text-[10px]">Ctrl+Enter</kbd> to execute</p>
          </div>
        </template>

        <!-- History tab -->
        <template v-else-if="activeTab === 'history'">
          <div v-if="queryHistory.length === 0" class="flex items-center justify-center h-full">
            <p class="text-sm text-slate-600">No queries yet</p>
          </div>
          <div v-else class="overflow-y-auto h-full divide-y divide-slate-800/50">
            <div
              v-for="(entry, i) in queryHistory"
              :key="i"
              class="flex items-start gap-3 px-4 py-3 hover:bg-slate-800/30 cursor-pointer group transition-colors"
              @click="applyHistory(entry.sql)"
            >
              <div
                class="mt-0.5 size-5 rounded-full flex items-center justify-center shrink-0"
                :class="entry.isError ? 'bg-rose-950 text-rose-400' : 'bg-emerald-950 text-emerald-400'"
              >
                <svg v-if="!entry.isError" class="size-2.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
                  <polyline points="20 6 9 17 4 12"/>
                </svg>
                <svg v-else class="size-2.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
                  <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
                </svg>
              </div>
              <div class="flex-1 min-w-0">
                <p class="text-xs font-mono text-slate-300 truncate">{{ entry.sql }}</p>
                <p class="text-[10px] text-slate-600 mt-0.5">
                  {{ entry.executionTime }}ms ·
                  {{ entry.result ? `${entry.result.rowCount} rows` : entry.message }}
                </p>
              </div>
              <button class="text-slate-700 group-hover:text-slate-400 transition-colors shrink-0 mt-0.5" title="Load query">
                <svg class="size-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <polyline points="9 14 4 9 9 4"/><path d="M20 20v-7a4 4 0 00-4-4H4"/>
                </svg>
              </button>
            </div>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>
