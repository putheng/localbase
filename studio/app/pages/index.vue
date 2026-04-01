<script setup lang="ts">
const {
  activeKeyspace,
  currentTables,
  currentTableData,
  activeTable,
  selectTable,
  dropTable,
} = useDatabase()

const showCreateModal = ref(false)
const tableSearch = ref('')
const confirmDrop = ref<string | null>(null)

const filteredTables = computed(() =>
  currentTables.value.filter(t =>
    t.name.toLowerCase().includes(tableSearch.value.toLowerCase()),
  ),
)

async function handleDrop(tableName: string) {
  await dropTable(activeKeyspace.value, tableName)
  confirmDrop.value = null
}

async function handleCreated(tableName: string) {
  await selectTable(tableName)
}
</script>

<template>
  <div class="flex h-full overflow-hidden">
    <!-- Left panel: table list -->
    <aside class="flex flex-col w-56 shrink-0 border-r border-slate-800 bg-slate-900/40">
      <!-- Header -->
      <div class="px-3 pt-4 pb-3 border-b border-slate-800 shrink-0">
        <div class="flex items-center justify-between mb-2.5">
          <span class="text-xs font-semibold text-slate-500 uppercase tracking-wider">Tables</span>
          <button
            @click="showCreateModal = true"
            class="flex items-center gap-1 text-xs text-violet-400 hover:text-violet-300 transition-colors px-1.5 py-0.5 rounded hover:bg-violet-500/10"
            title="New Table"
          >
            <svg class="size-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
              <line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>
            </svg>
            New
          </button>
        </div>
        <!-- Search -->
        <div class="relative">
          <svg class="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-slate-600" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
          </svg>
          <input
            v-model="tableSearch"
            type="text"
            placeholder="Search tables..."
            class="w-full bg-slate-800/80 border border-slate-700/50 rounded-md pl-7 pr-2.5 py-1.5 text-xs text-slate-300 placeholder-slate-600 focus:outline-none focus:border-violet-500/50 focus:ring-1 focus:ring-violet-500/30"
          />
        </div>
      </div>

      <!-- Table list -->
      <nav class="flex-1 overflow-y-auto py-1">
        <button
          v-for="table in filteredTables"
          :key="table.name"
          @click="selectTable(table.name)"
          class="flex items-center justify-between w-full px-3 py-2 text-left text-sm group transition-colors"
          :class="activeTable === table.name
            ? 'bg-violet-600/15 text-violet-300'
            : 'text-slate-400 hover:bg-slate-800/50 hover:text-slate-200'"
        >
          <div class="flex items-center gap-2 min-w-0">
            <svg class="size-3.5 shrink-0 opacity-60" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75">
              <rect x="3" y="3" width="18" height="18" rx="2"/>
              <path d="M3 9h18M9 3v18"/>
            </svg>
            <span class="truncate text-xs font-medium">{{ table.name }}</span>
          </div>
          <span class="text-[10px] text-slate-600 shrink-0 ml-1">{{ table.rows.length }}</span>
        </button>

        <div v-if="filteredTables.length === 0" class="px-3 py-8 text-center">
          <p class="text-xs text-slate-600">
            {{ tableSearch ? 'No tables match your search' : 'No tables yet' }}
          </p>
          <button v-if="!tableSearch" @click="showCreateModal = true" class="mt-2 text-xs text-violet-400 hover:text-violet-300">
            Create one →
          </button>
        </div>
      </nav>
    </aside>

    <!-- Right panel: table data -->
    <main class="flex-1 flex flex-col min-w-0 overflow-hidden">
      <template v-if="currentTableData">
        <!-- Table header toolbar -->
        <header class="flex items-center justify-between px-5 py-3 border-b border-slate-800 shrink-0 bg-slate-900/30">
          <div class="flex items-center gap-2 min-w-0">
            <span class="text-xs text-slate-500">{{ activeKeyspace }}</span>
            <svg class="size-3.5 text-slate-700 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <polyline points="9 18 15 12 9 6"/>
            </svg>
            <span class="text-sm font-semibold text-slate-100 truncate">{{ currentTableData.name }}</span>
            <span class="text-xs bg-slate-800 text-slate-500 px-1.5 py-0.5 rounded shrink-0">
              {{ currentTableData.rows.length }} rows
            </span>
          </div>
          <div class="flex items-center gap-2 shrink-0">
            <!-- Drop table -->
            <button
              @click="confirmDrop = currentTableData!.name"
              class="flex items-center gap-1.5 px-2.5 py-1.5 text-xs text-slate-500 hover:text-rose-400 hover:bg-rose-500/10 rounded-md border border-transparent hover:border-rose-500/20 transition-colors"
            >
              <svg class="size-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75">
                <polyline points="3 6 5 6 21 6"/>
                <path d="M19 6l-1 14a2 2 0 01-2 2H8a2 2 0 01-2-2L5 6"/>
                <path d="M10 11v6M14 11v6"/>
                <path d="M9 6V4a1 1 0 011-1h4a1 1 0 011 1v2"/>
              </svg>
              Drop
            </button>
          </div>
        </header>

        <!-- Data grid -->
        <div class="flex-1 overflow-hidden">
          <DataGrid :table="currentTableData" />
        </div>
      </template>

      <!-- Empty state: no table selected -->
      <template v-else>
        <div class="flex-1 flex flex-col items-center justify-center text-center p-8">
          <div class="size-16 rounded-2xl bg-slate-800/80 border border-slate-700/50 flex items-center justify-center mb-4">
            <svg class="size-7 text-slate-600" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <rect x="3" y="3" width="18" height="18" rx="2"/>
              <path d="M3 9h18M3 15h18M9 3v18"/>
            </svg>
          </div>
          <h2 class="text-base font-semibold text-slate-300 mb-1">No table selected</h2>
          <p class="text-sm text-slate-600 mb-5 max-w-xs">
            Select a table from the sidebar to view its data, or create a new one.
          </p>
          <button
            @click="showCreateModal = true"
            class="flex items-center gap-2 px-4 py-2 bg-violet-600 hover:bg-violet-500 text-white text-sm font-medium rounded-lg transition-colors"
          >
            <svg class="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>
            </svg>
            Create Table
          </button>
        </div>
      </template>
    </main>

    <!-- Drop confirm dialog -->
    <div v-if="confirmDrop" class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm">
      <div class="bg-slate-900 border border-slate-700 rounded-xl p-6 w-full max-w-sm shadow-2xl">
        <div class="flex items-start gap-3 mb-4">
          <div class="size-9 rounded-full bg-rose-500/10 border border-rose-500/20 flex items-center justify-center shrink-0">
            <svg class="size-4.5 text-rose-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/>
              <line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/>
            </svg>
          </div>
          <div>
            <h3 class="text-sm font-semibold text-slate-100">Drop table?</h3>
            <p class="text-xs text-slate-500 mt-1">
              This will permanently delete <span class="text-slate-300 font-medium">{{ confirmDrop }}</span> and all its data. This cannot be undone.
            </p>
          </div>
        </div>
        <div class="flex gap-2 justify-end">
          <button @click="confirmDrop = null" class="px-3 py-1.5 text-sm text-slate-400 hover:text-slate-200 hover:bg-slate-800 rounded-md transition-colors">
            Cancel
          </button>
          <button @click="handleDrop(confirmDrop!)" class="px-3 py-1.5 text-sm font-medium bg-rose-600 hover:bg-rose-500 text-white rounded-md transition-colors">
            Drop Table
          </button>
        </div>
      </div>
    </div>

    <!-- Create table modal -->
    <CreateTableModal
      v-if="showCreateModal"
      :keyspace-name="activeKeyspace"
      @close="showCreateModal = false"
      @created="handleCreated"
    />
  </div>
</template>
