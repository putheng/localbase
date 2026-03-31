<script setup lang="ts">
const route = useRoute()
const { keyspaces, activeKeyspace, selectKeyspace } = useDatabase()

const navItems = [
  { label: 'Table Editor', icon: 'table', to: '/' },
  { label: 'SQL Editor', icon: 'terminal', to: '/query' },
]

function isActive(to: string) {
  if (to === '/') return route.path === '/'
  return route.path.startsWith(to)
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
      <select
        :value="activeKeyspace"
        @change="selectKeyspace(($event.target as HTMLSelectElement).value)"
        class="w-full bg-slate-800 border border-slate-700 text-slate-200 text-sm rounded-md px-2.5 py-1.5 focus:outline-none focus:ring-1 focus:ring-violet-500 focus:border-violet-500 cursor-pointer"
      >
        <option v-for="ks in keyspaces" :key="ks.name" :value="ks.name">
          {{ ks.name }}
        </option>
      </select>
    </div>

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
