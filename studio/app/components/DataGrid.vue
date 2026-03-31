<script setup lang="ts">
import type { Table } from '~/composables/useDatabase'

const props = defineProps<{ table: Table }>()

const PAGE_SIZE = 50
const currentPage = ref(1)

const totalPages = computed(() => Math.max(1, Math.ceil(props.table.rows.length / PAGE_SIZE)))

const paginatedRows = computed(() => {
  const start = (currentPage.value - 1) * PAGE_SIZE
  return props.table.rows.slice(start, start + PAGE_SIZE)
})

watch(() => props.table.name, () => { currentPage.value = 1 })

function formatValue(val: unknown): string {
  if (val === null || val === undefined) return ''
  if (typeof val === 'boolean') return val ? 'true' : 'false'
  return String(val)
}

function isBool(val: unknown): boolean {
  return typeof val === 'boolean'
}
function isNull(val: unknown): boolean {
  return val === null || val === undefined
}

// ── Edit row ──────────────────────────────────────────────────────────────────
const editingRowIndex = ref<number | null>(null)
const editForm = ref<Record<string, string>>({})

function openEdit(pageRowIndex: number) {
  const absoluteIndex = (currentPage.value - 1) * PAGE_SIZE + pageRowIndex
  editingRowIndex.value = absoluteIndex
  // Copy current values as strings for the form
  const row = props.table.rows[absoluteIndex]!
  editForm.value = Object.fromEntries(
    props.table.columns.map(col => [col.name, formatValue(row[col.name])])
  )
}

function saveEdit() {
  if (editingRowIndex.value === null) return
  const row = props.table.rows[editingRowIndex.value]!
  props.table.columns.forEach(col => {
    const raw = editForm.value[col.name] ?? ''
    // Coerce back to original type
    const original = row[col.name]
    if (typeof original === 'boolean') {
      row[col.name] = raw.toLowerCase() === 'true'
    } else if (typeof original === 'number') {
      row[col.name] = Number(raw)
    } else {
      row[col.name] = raw
    }
  })
  closeEdit()
}

function closeEdit() {
  editingRowIndex.value = null
  editForm.value = {}
}
</script>

<template>
  <div class="flex flex-col h-full overflow-hidden">
    <!-- Column count info -->
    <div class="flex items-center gap-4 px-4 py-2 bg-slate-900/50 border-b border-slate-800 shrink-0 text-xs text-slate-500">
      <span>{{ table.columns.length }} columns</span>
      <span>{{ table.rows.length }} rows</span>
    </div>

    <!-- Table -->
    <div class="flex-1 overflow-auto">
      <table class="w-full border-collapse text-sm">
        <thead class="sticky top-0 z-10 bg-slate-900">
          <tr>
            <th class="px-3 py-2.5 border-b border-r border-slate-800 w-10 shrink-0"></th>
            <th
              v-for="col in table.columns"
              :key="col.name"
              class="px-4 py-2.5 text-left border-b border-r border-slate-800 whitespace-nowrap font-medium"
            >
              <div class="flex items-center gap-1.5">
                <span
                  v-if="col.isPartitionKey"
                  class="size-1.5 rounded-full bg-amber-400 shrink-0"
                  title="Partition key"
                ></span>
                <span
                  v-else-if="col.isClusteringKey"
                  class="size-1.5 rounded-full bg-blue-400 shrink-0"
                  title="Clustering key"
                ></span>
                <span class="text-slate-300">{{ col.name }}</span>
                <span class="text-slate-600 text-[10px] font-normal">{{ col.type }}</span>
              </div>
            </th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="(row, ri) in paginatedRows"
            :key="ri"
            class="border-b border-slate-800/50 hover:bg-slate-800/40 transition-colors group"
          >
            <!-- Edit button cell -->
            <td class="px-2 py-2 border-r border-slate-800/50 w-10">
              <button
                @click="openEdit(ri)"
                class="opacity-0 group-hover:opacity-100 flex items-center justify-center size-6 rounded hover:bg-violet-500/20 text-slate-600 hover:text-violet-400 transition-all"
                title="Edit row"
              >
                <svg class="size-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/>
                  <path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"/>
                </svg>
              </button>
            </td>
            <td
              v-for="col in table.columns"
              :key="col.name"
              class="px-4 py-2 border-r border-slate-800/50 max-w-xs"
            >
              <span v-if="isNull(row[col.name])" class="text-slate-600 italic text-xs">null</span>
              <span
                v-else-if="isBool(row[col.name])"
                class="inline-flex items-center gap-1 text-xs font-medium"
                :class="row[col.name] ? 'text-emerald-400' : 'text-rose-400'"
              >
                <span class="size-1.5 rounded-full inline-block" :class="row[col.name] ? 'bg-emerald-400' : 'bg-rose-400'"></span>
                {{ formatValue(row[col.name]) }}
              </span>
              <span v-else class="text-slate-300 font-mono text-xs truncate block" :title="formatValue(row[col.name])">
                {{ formatValue(row[col.name]) }}
              </span>
            </td>
          </tr>

          <tr v-if="paginatedRows.length === 0">
            <td :colspan="table.columns.length + 1" class="text-center py-16 text-slate-600">
              No rows in this table
            </td>
          </tr>
        </tbody>
      </table>
    </div>

  <!-- Edit Row Modal -->
  <Teleport to="body">
    <div
      v-if="editingRowIndex !== null"
      class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm"
      @click.self="closeEdit"
    >
      <div class="bg-slate-900 rounded-xl border border-slate-700 w-full max-w-lg max-h-[85vh] flex flex-col shadow-2xl">
        <!-- Header -->
        <div class="flex items-center justify-between px-6 py-4 border-b border-slate-800 shrink-0">
          <div>
            <h2 class="text-base font-semibold text-slate-100">Edit Row</h2>
            <p class="text-xs text-slate-500 mt-0.5">
              <span class="text-violet-400">{{ table.name }}</span> · row {{ editingRowIndex! + 1 }}
            </p>
          </div>
          <button @click="closeEdit" class="text-slate-500 hover:text-slate-300 transition-colors p-1 rounded">
            <svg class="size-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
            </svg>
          </button>
        </div>

        <!-- Fields -->
        <div class="flex-1 overflow-y-auto px-6 py-4 flex flex-col gap-3">
          <div v-for="col in table.columns" :key="col.name">
            <label class="flex items-center gap-1.5 text-xs font-medium text-slate-400 mb-1">
              <span
                v-if="col.isPartitionKey"
                class="size-1.5 rounded-full bg-amber-400 inline-block"
                title="Partition key"
              ></span>
              <span
                v-else-if="col.isClusteringKey"
                class="size-1.5 rounded-full bg-blue-400 inline-block"
                title="Clustering key"
              ></span>
              {{ col.name }}
              <span class="text-slate-600 font-normal">{{ col.type }}</span>
              <span v-if="col.isPartitionKey" class="text-amber-500/70 text-[10px] font-normal">(partition key)</span>
              <span v-else-if="col.isClusteringKey" class="text-blue-500/70 text-[10px] font-normal">(clustering key)</span>
            </label>
            <input
              v-model="editForm[col.name]"
              type="text"
              :placeholder="'null'"
              class="w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-sm font-mono text-slate-100 placeholder-slate-700 focus:outline-none focus:ring-1 focus:ring-violet-500 focus:border-violet-500 transition-colors"
              :class="col.isPartitionKey || col.isClusteringKey ? 'opacity-60 cursor-not-allowed' : ''"
              :readonly="col.isPartitionKey || col.isClusteringKey"
            />
          </div>
        </div>

        <!-- Footer -->
        <div class="flex items-center justify-end gap-2.5 px-6 py-4 border-t border-slate-800 shrink-0">
          <button
            @click="closeEdit"
            class="px-4 py-2 text-sm text-slate-400 hover:text-slate-200 transition-colors rounded-lg hover:bg-slate-800"
          >
            Cancel
          </button>
          <button
            @click="saveEdit"
            class="px-4 py-2 text-sm font-medium bg-violet-600 hover:bg-violet-500 text-white rounded-lg transition-colors"
          >
            Save Changes
          </button>
        </div>
      </div>
    </div>
  </Teleport>

    <!-- Pagination -->
    <div v-if="totalPages > 1" class="flex items-center justify-between px-4 py-2.5 border-t border-slate-800 shrink-0 bg-slate-900/50">
      <span class="text-xs text-slate-500">
        Page {{ currentPage }} of {{ totalPages }} · {{ table.rows.length }} total rows
      </span>
      <div class="flex items-center gap-1">
        <button
          @click="currentPage--"
          :disabled="currentPage <= 1"
          class="px-2.5 py-1 text-xs rounded border border-slate-700 text-slate-400 hover:text-slate-200 hover:border-slate-600 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
        >
          ← Prev
        </button>
        <button
          @click="currentPage++"
          :disabled="currentPage >= totalPages"
          class="px-2.5 py-1 text-xs rounded border border-slate-700 text-slate-400 hover:text-slate-200 hover:border-slate-600 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
        >
          Next →
        </button>
      </div>
    </div>
  </div>
</template>
