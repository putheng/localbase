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

// ── Security Rules ────────────────────────────────────────────────────────────
const showSecurityRules = ref(false)

const OPERATIONS = ['SELECT', 'INSERT', 'UPDATE', 'DELETE'] as const
type Operation = typeof OPERATIONS[number]

const TARGETS = ['anon', 'authenticated'] as const
type Target = typeof TARGETS[number]

interface SecurityRule {
  id: number
  target: Target
  role: string
  operations: Operation[]
  condition: string
  description: string
}

const rules = ref<SecurityRule[]>([
  { id: 1, target: 'authenticated', role: 'admin', operations: ['SELECT', 'INSERT', 'UPDATE', 'DELETE'], condition: '', description: 'Full access for admins' },
  { id: 2, target: 'anon', role: 'readonly', operations: ['SELECT'], condition: '', description: 'Read-only access' },
])

const newRule = ref({ target: 'anon' as Target, role: '', operations: [] as Operation[], condition: '', description: '' })
const ruleError = ref('')

function toggleOperation(op: Operation) {
  const idx = newRule.value.operations.indexOf(op)
  if (idx === -1) newRule.value.operations.push(op)
  else newRule.value.operations.splice(idx, 1)
}

function addRule() {
  ruleError.value = ''
  if (!newRule.value.role.trim()) { ruleError.value = 'Role name is required'; return }
  if (newRule.value.operations.length === 0) { ruleError.value = 'Select at least one operation'; return }
  rules.value.push({
    id: Date.now(),
    target: newRule.value.target,
    role: newRule.value.role.trim(),
    operations: [...newRule.value.operations],
    condition: newRule.value.condition.trim(),
    description: newRule.value.description.trim(),
  })
  newRule.value = { target: 'anon', role: '', operations: [], condition: '', description: '' }
}

function removeRule(id: number) {
  rules.value = rules.value.filter(r => r.id !== id)
}

const OP_COLOR: Record<Operation, string> = {
  SELECT: 'bg-blue-500/15 text-blue-300 border-blue-500/30',
  INSERT: 'bg-emerald-500/15 text-emerald-300 border-emerald-500/30',
  UPDATE: 'bg-amber-500/15 text-amber-300 border-amber-500/30',
  DELETE: 'bg-rose-500/15 text-rose-300 border-rose-500/30',
}
</script>

<template>
  <div class="flex flex-col h-full overflow-hidden">
    <!-- Toolbar -->
    <div class="flex items-center justify-between px-4 py-2 bg-slate-900/50 border-b border-slate-800 shrink-0">
      <div class="flex items-center gap-3 text-xs text-slate-500">
        <span>{{ table.columns.length }} columns</span>
        <span class="text-slate-700">·</span>
        <span>{{ table.rows.length }} rows</span>
      </div>
      <button
        @click="showSecurityRules = true"
        class="flex items-center gap-1.5 px-2.5 py-1.5 text-xs font-medium text-slate-400 hover:text-slate-200 bg-slate-800/60 hover:bg-slate-800 border border-slate-700/60 hover:border-slate-600 rounded-md transition-colors"
      >
        <svg class="size-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75">
          <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
        </svg>
        Security Rules
        <span class="ml-0.5 size-4 flex items-center justify-center rounded-full bg-violet-500/20 text-violet-400 text-[10px] font-semibold">{{ rules.length }}</span>
      </button>
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

  <!-- Security Rules Modal -->
  <Teleport to="body">
    <div
      v-if="showSecurityRules"
      class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm"
      @click.self="showSecurityRules = false"
    >
      <div class="bg-slate-900 rounded-xl border border-slate-700 w-full max-w-2xl max-h-[88vh] flex flex-col shadow-2xl">
        <!-- Header -->
        <div class="flex items-center justify-between px-6 py-4 border-b border-slate-800 shrink-0">
          <div class="flex items-center gap-2.5">
            <div class="size-8 rounded-lg bg-violet-600/20 border border-violet-500/30 flex items-center justify-center">
              <svg class="size-4 text-violet-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75">
                <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
              </svg>
            </div>
            <div>
              <h2 class="text-base font-semibold text-slate-100">Security Rules</h2>
              <p class="text-xs text-slate-500 mt-0.5">Table: <span class="text-violet-400">{{ table.name }}</span></p>
            </div>
          </div>
          <button @click="showSecurityRules = false" class="text-slate-500 hover:text-slate-300 transition-colors p-1 rounded">
            <svg class="size-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
            </svg>
          </button>
        </div>

        <div class="flex-1 overflow-y-auto">
          <!-- Existing rules -->
          <div class="px-6 pt-4 pb-2">
            <p class="text-[10px] font-semibold uppercase tracking-wider text-slate-600 mb-2">Active Rules</p>
            <div v-if="rules.length === 0" class="text-xs text-slate-600 py-4 text-center border border-dashed border-slate-800 rounded-lg">
              No rules defined. All operations are allowed.
            </div>
            <div v-else class="flex flex-col gap-2">
              <div
                v-for="rule in rules"
                :key="rule.id"
                class="flex items-start gap-3 bg-slate-800/50 border border-slate-700/50 rounded-lg px-4 py-3"
              >
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-2 flex-wrap">
                    <span
                      class="text-[10px] font-semibold px-1.5 py-0.5 rounded border"
                      :class="rule.target === 'authenticated'
                        ? 'bg-violet-500/15 text-violet-300 border-violet-500/30'
                        : 'bg-slate-500/15 text-slate-400 border-slate-500/30'"
                    >{{ rule.target }}</span>
                    <span class="text-sm font-medium text-slate-200">{{ rule.role }}</span>
                    <div class="flex items-center gap-1 flex-wrap">
                      <span
                        v-for="op in rule.operations"
                        :key="op"
                        class="text-[10px] font-semibold px-1.5 py-0.5 rounded border"
                        :class="OP_COLOR[op]"
                      >{{ op }}</span>
                    </div>
                  </div>
                  <p v-if="rule.condition" class="text-xs text-slate-500 font-mono mt-1 truncate">WHERE {{ rule.condition }}</p>
                  <p v-if="rule.description" class="text-xs text-slate-600 mt-0.5">{{ rule.description }}</p>
                </div>
                <button
                  @click="removeRule(rule.id)"
                  class="shrink-0 size-6 flex items-center justify-center text-slate-700 hover:text-rose-400 hover:bg-rose-500/10 rounded transition-colors"
                  title="Remove rule"
                >
                  <svg class="size-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <polyline points="3 6 5 6 21 6"/>
                    <path d="M19 6l-1 14a2 2 0 01-2 2H8a2 2 0 01-2-2L5 6"/>
                    <path d="M10 11v6M14 11v6"/>
                  </svg>
                </button>
              </div>
            </div>
          </div>

          <!-- Add new rule -->
          <div class="px-6 pt-4 pb-6">
            <p class="text-[10px] font-semibold uppercase tracking-wider text-slate-600 mb-3">Add Rule</p>
            <div class="bg-slate-800/30 border border-slate-700/50 rounded-lg p-4 flex flex-col gap-3">
              <!-- Target -->
              <div>
                <label class="block text-xs font-medium text-slate-400 mb-1">Target</label>
                <div class="flex gap-2">
                  <button
                    v-for="t in TARGETS"
                    :key="t"
                    @click="newRule.target = t"
                    class="flex-1 py-1.5 text-xs font-semibold rounded-md border transition-colors"
                    :class="newRule.target === t
                      ? t === 'authenticated'
                        ? 'bg-violet-500/15 text-violet-300 border-violet-500/40 ring-1 ring-inset ring-violet-500/30'
                        : 'bg-slate-500/15 text-slate-300 border-slate-500/40 ring-1 ring-inset ring-slate-500/30'
                      : 'bg-slate-900 text-slate-600 border-slate-700 hover:border-slate-500 hover:text-slate-400'"
                  >
                    {{ t }}
                  </button>
                </div>
              </div>

              <!-- Role -->
              <div>
                <label class="block text-xs font-medium text-slate-400 mb-1">Role / Principal</label>
                <input
                  v-model="newRule.role"
                  type="text"
                  placeholder="e.g. admin, service_account, readonly"
                  class="w-full bg-slate-900 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-100 placeholder-slate-600 focus:outline-none focus:ring-1 focus:ring-violet-500 focus:border-violet-500"
                />
              </div>

              <!-- Operations -->
              <div>
                <label class="block text-xs font-medium text-slate-400 mb-2">Allowed CQL Operations</label>
                <div class="flex items-center gap-2">
                  <button
                    v-for="op in OPERATIONS"
                    :key="op"
                    @click="toggleOperation(op)"
                    class="px-3 py-1.5 text-xs font-semibold rounded-md border transition-colors"
                    :class="newRule.operations.includes(op)
                      ? OP_COLOR[op] + ' ring-1 ring-inset ring-current'
                      : 'bg-slate-900 text-slate-600 border-slate-700 hover:border-slate-500 hover:text-slate-400'"
                  >
                    {{ op }}
                  </button>
                </div>
              </div>

              <!-- Condition -->
              <div>
                <label class="block text-xs font-medium text-slate-400 mb-1">
                  Row-level Condition
                  <span class="text-slate-600 font-normal ml-1">(optional · CQL WHERE expression)</span>
                </label>
                <div class="relative">
                  <span class="absolute left-3 top-1/2 -translate-y-1/2 text-xs text-slate-600 font-mono select-none">WHERE</span>
                  <input
                    v-model="newRule.condition"
                    type="text"
                    placeholder="user_id = current_user()"
                    class="w-full bg-slate-900 border border-slate-700 rounded-lg pl-16 pr-3 py-2 text-sm font-mono text-slate-100 placeholder-slate-600 focus:outline-none focus:ring-1 focus:ring-violet-500 focus:border-violet-500"
                  />
                </div>
              </div>

              <!-- Description -->
              <div>
                <label class="block text-xs font-medium text-slate-400 mb-1">
                  Description
                  <span class="text-slate-600 font-normal ml-1">(optional)</span>
                </label>
                <input
                  v-model="newRule.description"
                  type="text"
                  placeholder="Short description of this rule"
                  class="w-full bg-slate-900 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-100 placeholder-slate-600 focus:outline-none focus:ring-1 focus:ring-violet-500 focus:border-violet-500"
                />
              </div>

              <!-- Error -->
              <div v-if="ruleError" class="flex items-center gap-2 bg-rose-950/50 border border-rose-800 rounded-lg px-3 py-2">
                <svg class="size-3.5 text-rose-400 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/>
                </svg>
                <span class="text-xs text-rose-300">{{ ruleError }}</span>
              </div>

              <button
                @click="addRule"
                class="self-end flex items-center gap-1.5 px-4 py-2 text-sm font-medium bg-violet-600 hover:bg-violet-500 text-white rounded-lg transition-colors"
              >
                <svg class="size-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                  <line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>
                </svg>
                Add Rule
              </button>
            </div>
          </div>
        </div>

        <!-- Footer -->
        <div class="flex items-center justify-end px-6 py-3 border-t border-slate-800 shrink-0">
          <button
            @click="showSecurityRules = false"
            class="px-4 py-2 text-sm font-medium bg-slate-800 hover:bg-slate-700 text-slate-200 rounded-lg transition-colors"
          >
            Done
          </button>
        </div>
      </div>
    </div>
  </Teleport>

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
