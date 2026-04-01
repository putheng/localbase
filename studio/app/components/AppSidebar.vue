<script setup lang="ts">
const route = useRoute()
const { keyspaces, activeKeyspace, isLoadingKeyspaces, loadKeyspaces, selectKeyspace, createKeyspace } = useDatabase()

onMounted(async () => {
  await loadKeyspaces()
})

const navItems = [
  { label: 'Table Editor', icon: 'table', to: '/' },
  { label: 'SQL Editor', icon: 'terminal', to: '/query' },
  { label: 'Storage', icon: 'storage', to: '/storage' },
]

function isActive(to: string) {
  if (to === '/') return route.path === '/'
  return route.path.startsWith(to)
}

// ── Create Keyspace dialog ──────────────────────────────────────────────────
const showCreateDialog = ref(false)
const newKsName = ref('')
const newKsRf = ref(1)
const isCreating = ref(false)
const createError = ref('')

function openCreate() {
  newKsName.value = ''
  newKsRf.value = 1
  createError.value = ''
  showCreateDialog.value = true
}

function closeCreate() {
  showCreateDialog.value = false
}

async function handleCreate() {
  const name = newKsName.value.trim()
  if (!name) { createError.value = 'Keyspace name is required.'; return }
  if (!/^[a-zA-Z][a-zA-Z0-9_]*$/.test(name)) { createError.value = 'Only letters, digits, and underscores (must start with a letter).'; return }
  isCreating.value = true
  createError.value = ''
  try {
    await createKeyspace(name, newKsRf.value)
    showCreateDialog.value = false
  } catch (e: unknown) {
    createError.value = e instanceof Error ? e.message : String(e)
  } finally {
    isCreating.value = false
  }
}
</script>

<template>
  <aside class="flex flex-col w-60 shrink-0 bg-slate-900 border-r border-slate-800 h-screen">
    <!-- Logo -->
    <div class="flex items-center gap-2.5 px-4 h-14 border-b border-slate-800 shrink-0">
      <div class="flex size-8 items-center justify-center rounded-lg bg-violet-600 shrink-0">
        <svg class="size-4.5 text-white" viewBox="0 0 24 24" fill="currentColor">
          <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 14H9V8h2v8zm4 0h-2V8h2v8z"/>
        </svg>
      </div>
      <div class="min-w-0">
        <p class="text-sm font-semibold text-slate-100 truncate">ScyllaDB Studio</p>
        <p class="text-xs text-slate-500 truncate">localhost:9042</p>
      </div>
    </div>

    <!-- Keyspace selector -->
    <div class="px-3 py-3 border-b border-slate-800 shrink-0">
      <label class="block text-[10px] font-semibold uppercase tracking-wider text-slate-500 mb-1.5 px-1">Keyspace</label>
      <div class="flex gap-1.5">
        <select
          :value="activeKeyspace"
          :disabled="isLoadingKeyspaces"
          @change="selectKeyspace(($event.target as HTMLSelectElement).value)"
          class="flex-1 min-w-0 bg-slate-800 border border-slate-700 text-slate-200 text-sm rounded-md px-2.5 py-1.5 focus:outline-none focus:ring-1 focus:ring-violet-500 focus:border-violet-500 cursor-pointer disabled:opacity-50 disabled:cursor-wait"
        >
          <option v-if="isLoadingKeyspaces" disabled value="">Loading…</option>
          <option v-for="ks in keyspaces" :key="ks.name" :value="ks.name">
            {{ ks.name }}
          </option>
        </select>
        <button
          @click="openCreate"
          title="Create keyspace"
          class="shrink-0 flex items-center justify-center size-7.5 rounded-md bg-slate-800 border border-slate-700 text-slate-400 hover:text-violet-300 hover:border-violet-500 transition-colors"
        >
          <svg class="size-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
            <path d="M12 5v14M5 12h14"/>
          </svg>
        </button>
      </div>
    </div>

    <!-- Create Keyspace dialog -->
    <Teleport to="body">
      <Transition
        enter-active-class="transition duration-150 ease-out"
        enter-from-class="opacity-0"
        enter-to-class="opacity-100"
        leave-active-class="transition duration-100 ease-in"
        leave-from-class="opacity-100"
        leave-to-class="opacity-0"
      >
        <div v-if="showCreateDialog" class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60" @click.self="closeCreate">
          <div class="w-full max-w-sm bg-slate-900 border border-slate-700 rounded-xl shadow-2xl">
            <!-- Header -->
            <div class="flex items-center justify-between px-5 py-4 border-b border-slate-800">
              <h2 class="text-sm font-semibold text-slate-100">Create Keyspace</h2>
              <button @click="closeCreate" class="text-slate-500 hover:text-slate-300 transition-colors">
                <svg class="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M18 6L6 18M6 6l12 12"/>
                </svg>
              </button>
            </div>

            <!-- Form -->
            <div class="px-5 py-4 space-y-4">
              <div>
                <label class="block text-xs font-medium text-slate-400 mb-1.5">Keyspace Name</label>
                <input
                  v-model="newKsName"
                  type="text"
                  placeholder="my_keyspace"
                  :disabled="isCreating"
                  @keydown.enter="handleCreate"
                  class="w-full bg-slate-800 border border-slate-700 text-slate-200 text-sm rounded-md px-3 py-2 placeholder-slate-600 focus:outline-none focus:ring-1 focus:ring-violet-500 focus:border-violet-500 disabled:opacity-50"
                />
              </div>
              <div>
                <label class="block text-xs font-medium text-slate-400 mb-1.5">Replication Factor</label>
                <input
                  v-model.number="newKsRf"
                  type="number"
                  min="1"
                  max="9"
                  :disabled="isCreating"
                  class="w-full bg-slate-800 border border-slate-700 text-slate-200 text-sm rounded-md px-3 py-2 focus:outline-none focus:ring-1 focus:ring-violet-500 focus:border-violet-500 disabled:opacity-50"
                />
                <p class="mt-1 text-[11px] text-slate-500">SimpleStrategy — sets the number of data replicas.</p>
              </div>
              <p v-if="createError" class="text-xs text-rose-400">{{ createError }}</p>
            </div>

            <!-- Footer -->
            <div class="flex justify-end gap-2 px-5 py-4 border-t border-slate-800">
              <button
                @click="closeCreate"
                :disabled="isCreating"
                class="px-3 py-1.5 text-sm text-slate-400 hover:text-slate-200 transition-colors disabled:opacity-50"
              >Cancel</button>
              <button
                @click="handleCreate"
                :disabled="isCreating"
                class="px-4 py-1.5 text-sm font-medium bg-violet-600 hover:bg-violet-500 text-white rounded-md transition-colors disabled:opacity-50 disabled:cursor-wait flex items-center gap-1.5"
              >
                <svg v-if="isCreating" class="size-3.5 animate-spin" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                  <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"/>
                </svg>
                {{ isCreating ? 'Creating…' : 'Create' }}
              </button>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>

    <!-- Navigation -->
    <nav class="flex flex-col gap-0.5 px-2 py-3 flex-1 overflow-y-auto">
      <NuxtLink
        v-for="item in navItems"
        :key="item.to"
        :to="item.to"
        class="flex items-center gap-2.5 px-3 py-2 rounded-md text-sm transition-colors"
        :class="isActive(item.to)
          ? 'bg-violet-600/20 text-violet-300 font-medium'
          : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800'"
      >
        <!-- Table icon -->
        <svg v-if="item.icon === 'table'" class="size-4 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75">
          <rect x="3" y="3" width="18" height="18" rx="2"/>
          <path d="M3 9h18M3 15h18M9 3v18"/>
        </svg>
        <!-- Terminal icon -->
        <svg v-else-if="item.icon === 'terminal'" class="size-4 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75">
          <polyline points="4 17 10 11 4 5"/>
          <line x1="12" y1="19" x2="20" y2="19"/>
        </svg>
        <!-- Storage icon -->
        <svg v-else-if="item.icon === 'storage'" class="size-4 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75">
          <ellipse cx="12" cy="5" rx="9" ry="3"/>
          <path d="M21 12c0 1.66-4.03 3-9 3S3 13.66 3 12"/>
          <path d="M3 5v14c0 1.66 4.03 3 9 3s9-1.34 9-3V5"/>
        </svg>
        {{ item.label }}
      </NuxtLink>
    </nav>

    <!-- Footer / connection status -->
    <div class="px-4 py-3 border-t border-slate-800 shrink-0">
      <div class="flex items-center gap-2">
        <span class="size-2 rounded-full bg-emerald-500 shrink-0 shadow-[0_0_6px_#10b981]"></span>
        <span class="text-xs text-slate-400">Connected · ScyllaDB 5.4.0</span>
      </div>
    </div>
  </aside>
</template>
