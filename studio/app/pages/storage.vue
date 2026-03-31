<script setup lang="ts">
interface StorageFile {
  id: string
  name: string
  size: number
  type: string
  lastModified: string
  path: string
}

interface Bucket {
  id: string
  name: string
  public: boolean
  files: StorageFile[]
}

const buckets = ref<Bucket[]>([
  {
    id: 'b1',
    name: 'avatars',
    public: true,
    files: [
      { id: 'f1', name: 'alice.png', size: 42300, type: 'image/png', lastModified: '2024-05-01 10:00', path: 'avatars/alice.png' },
      { id: 'f2', name: 'bob.jpg', size: 87500, type: 'image/jpeg', lastModified: '2024-05-02 14:30', path: 'avatars/bob.jpg' },
      { id: 'f3', name: 'carol.webp', size: 31200, type: 'image/webp', lastModified: '2024-05-03 09:15', path: 'avatars/carol.webp' },
    ],
  },
  {
    id: 'b2',
    name: 'documents',
    public: false,
    files: [
      { id: 'f4', name: 'invoice-2024-01.pdf', size: 204800, type: 'application/pdf', lastModified: '2024-01-31 17:00', path: 'documents/invoice-2024-01.pdf' },
      { id: 'f5', name: 'contract.docx', size: 512000, type: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document', lastModified: '2024-03-10 11:45', path: 'documents/contract.docx' },
      { id: 'f6', name: 'report-q1.xlsx', size: 388096, type: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet', lastModified: '2024-04-01 08:00', path: 'documents/report-q1.xlsx' },
      { id: 'f7', name: 'notes.txt', size: 1240, type: 'text/plain', lastModified: '2024-05-05 20:12', path: 'documents/notes.txt' },
    ],
  },
  {
    id: 'b3',
    name: 'media',
    public: false,
    files: [
      { id: 'f8', name: 'intro-video.mp4', size: 52428800, type: 'video/mp4', lastModified: '2024-04-20 13:00', path: 'media/intro-video.mp4' },
      { id: 'f9', name: 'banner.png', size: 128000, type: 'image/png', lastModified: '2024-04-22 10:30', path: 'media/banner.png' },
    ],
  },
])

const activeBucketId = ref('b1')
const fileSearch = ref('')
const selectedFileIds = ref<Set<string>>(new Set())
const showNewBucketModal = ref(false)
const newBucketName = ref('')
const newBucketPublic = ref(false)
const newBucketError = ref('')
const confirmDeleteBucket = ref<string | null>(null)
const confirmDeleteFile = ref<string | null>(null)
const isDraggingOver = ref(false)

const activeBucket = computed(() => buckets.value.find(b => b.id === activeBucketId.value))

const filteredFiles = computed(() =>
  (activeBucket.value?.files ?? []).filter(f =>
    f.name.toLowerCase().includes(fileSearch.value.toLowerCase()),
  ),
)

const allSelected = computed(() =>
  filteredFiles.value.length > 0 && filteredFiles.value.every(f => selectedFileIds.value.has(f.id)),
)

function toggleSelectAll() {
  if (allSelected.value) {
    filteredFiles.value.forEach(f => selectedFileIds.value.delete(f.id))
  } else {
    filteredFiles.value.forEach(f => selectedFileIds.value.add(f.id))
  }
}

function toggleFile(id: string) {
  if (selectedFileIds.value.has(id)) selectedFileIds.value.delete(id)
  else selectedFileIds.value.add(id)
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`
  if (bytes < 1073741824) return `${(bytes / 1048576).toFixed(1)} MB`
  return `${(bytes / 1073741824).toFixed(2)} GB`
}

function fileIcon(type: string): 'image' | 'video' | 'pdf' | 'text' | 'file' {
  if (type.startsWith('image/')) return 'image'
  if (type.startsWith('video/')) return 'video'
  if (type === 'application/pdf') return 'pdf'
  if (type.startsWith('text/')) return 'text'
  return 'file'
}

function addBucket() {
  newBucketError.value = ''
  const name = newBucketName.value.trim().toLowerCase().replace(/\s+/g, '-')
  if (!name) { newBucketError.value = 'Bucket name is required'; return }
  if (!/^[a-z0-9-]+$/.test(name)) { newBucketError.value = 'Only lowercase letters, numbers, and hyphens allowed'; return }
  if (buckets.value.find(b => b.name === name)) { newBucketError.value = 'Bucket name already exists'; return }
  const id = `b-${Date.now()}`
  buckets.value.push({ id, name, public: newBucketPublic.value, files: [] })
  activeBucketId.value = id
  newBucketName.value = ''
  newBucketPublic.value = false
  showNewBucketModal.value = false
}

function deleteBucket(id: string) {
  buckets.value = buckets.value.filter(b => b.id !== id)
  if (activeBucketId.value === id) activeBucketId.value = buckets.value[0]?.id ?? ''
  confirmDeleteBucket.value = null
}

function deleteFile(fileId: string) {
  if (!activeBucket.value) return
  activeBucket.value.files = activeBucket.value.files.filter(f => f.id !== fileId)
  selectedFileIds.value.delete(fileId)
  confirmDeleteFile.value = null
}

function deleteSelected() {
  if (!activeBucket.value) return
  activeBucket.value.files = activeBucket.value.files.filter(f => !selectedFileIds.value.has(f.id))
  selectedFileIds.value.clear()
}

function simulateUpload(names: string[]) {
  if (!activeBucket.value) return
  names.forEach(name => {
    activeBucket.value!.files.push({
      id: `f-${Date.now()}-${Math.random()}`,
      name,
      size: Math.floor(Math.random() * 500000) + 5000,
      type: guessType(name),
      lastModified: new Date().toISOString().replace('T', ' ').slice(0, 16),
      path: `${activeBucket.value!.name}/${name}`,
    })
  })
}

function guessType(name: string): string {
  const ext = name.split('.').pop()?.toLowerCase()
  const map: Record<string, string> = {
    png: 'image/png', jpg: 'image/jpeg', jpeg: 'image/jpeg', webp: 'image/webp', gif: 'image/gif',
    mp4: 'video/mp4', mov: 'video/quicktime',
    pdf: 'application/pdf', txt: 'text/plain',
    docx: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
    xlsx: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
  }
  return map[ext ?? ''] ?? 'application/octet-stream'
}

function onDropFiles(e: DragEvent) {
  isDraggingOver.value = false
  const files = Array.from(e.dataTransfer?.files ?? [])
  if (files.length) simulateUpload(files.map(f => f.name))
}

function onFileInputChange(e: Event) {
  const files = Array.from((e.target as HTMLInputElement).files ?? [])
  if (files.length) simulateUpload(files.map(f => f.name))
  ;(e.target as HTMLInputElement).value = ''
}
</script>

<template>
  <div class="flex h-full overflow-hidden">
    <!-- Bucket sidebar -->
    <aside class="flex flex-col w-52 shrink-0 border-r border-slate-800 bg-slate-900/40">
      <div class="px-3 pt-4 pb-3 border-b border-slate-800 shrink-0">
        <div class="flex items-center justify-between mb-1">
          <span class="text-xs font-semibold text-slate-500 uppercase tracking-wider">Buckets</span>
          <button
            @click="showNewBucketModal = true"
            class="flex items-center gap-1 text-xs text-violet-400 hover:text-violet-300 transition-colors px-1.5 py-0.5 rounded hover:bg-violet-500/10"
          >
            <svg class="size-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
              <line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>
            </svg>
            New
          </button>
        </div>
      </div>

      <nav class="flex-1 overflow-y-auto py-1">
        <button
          v-for="bucket in buckets"
          :key="bucket.id"
          @click="activeBucketId = bucket.id; selectedFileIds.clear(); fileSearch = ''"
          class="flex items-center justify-between w-full px-3 py-2 text-left group transition-colors"
          :class="activeBucketId === bucket.id
            ? 'bg-violet-600/15 text-violet-300'
            : 'text-slate-400 hover:bg-slate-800/50 hover:text-slate-200'"
        >
          <div class="flex items-center gap-2 min-w-0">
            <svg class="size-3.5 shrink-0 opacity-70" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75">
              <ellipse cx="12" cy="5" rx="9" ry="3"/>
              <path d="M21 12c0 1.66-4.03 3-9 3S3 13.66 3 12"/>
              <path d="M3 5v14c0 1.66 4.03 3 9 3s9-1.34 9-3V5"/>
            </svg>
            <span class="text-xs font-medium truncate">{{ bucket.name }}</span>
          </div>
          <div class="flex items-center gap-1.5 shrink-0 ml-1">
            <span
              class="text-[9px] px-1 py-0.5 rounded font-semibold"
              :class="bucket.public ? 'bg-emerald-500/15 text-emerald-400' : 'bg-slate-700 text-slate-500'"
            >{{ bucket.public ? 'pub' : 'priv' }}</span>
            <span class="text-[10px] text-slate-600">{{ bucket.files.length }}</span>
          </div>
        </button>
      </nav>
    </aside>

    <!-- Main content -->
    <main class="flex-1 flex flex-col min-w-0 overflow-hidden">
      <template v-if="activeBucket">
        <!-- Toolbar -->
        <header class="flex items-center gap-3 px-5 py-3 border-b border-slate-800 bg-slate-900/30 shrink-0">
          <div class="flex items-center gap-2 min-w-0 flex-1">
            <span class="text-sm font-semibold text-slate-100 truncate">{{ activeBucket.name }}</span>
            <span
              class="text-xs px-1.5 py-0.5 rounded font-semibold shrink-0"
              :class="activeBucket.public ? 'bg-emerald-500/15 text-emerald-400' : 'bg-slate-700 text-slate-500'"
            >{{ activeBucket.public ? 'Public' : 'Private' }}</span>
            <span class="text-xs text-slate-600 shrink-0">{{ activeBucket.files.length }} files</span>
          </div>

          <!-- Search -->
          <div class="relative w-48 shrink-0">
            <svg class="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-slate-600 pointer-events-none" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
            </svg>
            <input
              v-model="fileSearch"
              type="text"
              placeholder="Search files…"
              class="w-full bg-slate-800/80 border border-slate-700/50 rounded-md pl-7 pr-2.5 py-1.5 text-xs text-slate-300 placeholder-slate-600 focus:outline-none focus:border-violet-500/50 focus:ring-1 focus:ring-violet-500/30"
            />
          </div>

          <!-- Delete selected -->
          <button
            v-if="selectedFileIds.size > 0"
            @click="deleteSelected"
            class="flex items-center gap-1.5 px-2.5 py-1.5 text-xs text-rose-400 hover:bg-rose-500/10 rounded-md border border-rose-500/20 transition-colors shrink-0"
          >
            <svg class="size-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75">
              <polyline points="3 6 5 6 21 6"/>
              <path d="M19 6l-1 14a2 2 0 01-2 2H8a2 2 0 01-2-2L5 6"/>
            </svg>
            Delete ({{ selectedFileIds.size }})
          </button>

          <!-- Upload button -->
          <label class="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium bg-violet-600 hover:bg-violet-500 text-white rounded-md transition-colors cursor-pointer shrink-0">
            <svg class="size-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"/>
              <polyline points="17 8 12 3 7 8"/>
              <line x1="12" y1="3" x2="12" y2="15"/>
            </svg>
            Upload
            <input type="file" multiple class="hidden" @change="onFileInputChange" />
          </label>

          <!-- Delete bucket -->
          <button
            @click="confirmDeleteBucket = activeBucket.id"
            class="flex items-center gap-1.5 px-2.5 py-1.5 text-xs text-slate-500 hover:text-rose-400 hover:bg-rose-500/10 rounded-md border border-transparent hover:border-rose-500/20 transition-colors shrink-0"
            title="Delete bucket"
          >
            <svg class="size-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75">
              <ellipse cx="12" cy="5" rx="9" ry="3"/>
              <path d="M21 12c0 1.66-4.03 3-9 3S3 13.66 3 12"/>
              <path d="M3 5v14c0 1.66 4.03 3 9 3s9-1.34 9-3V5"/>
              <line x1="4" y1="4" x2="20" y2="20" stroke-width="2"/>
            </svg>
          </button>
        </header>

        <!-- Drop zone + file list -->
        <div
          class="flex-1 overflow-hidden flex flex-col relative"
          @dragover.prevent="isDraggingOver = true"
          @dragleave="isDraggingOver = false"
          @drop.prevent="onDropFiles"
        >
          <!-- Drop overlay -->
          <div
            v-if="isDraggingOver"
            class="absolute inset-0 z-20 bg-violet-600/10 border-2 border-dashed border-violet-500 rounded flex items-center justify-center pointer-events-none"
          >
            <p class="text-sm font-medium text-violet-300">Drop files to upload</p>
          </div>

          <!-- File table -->
          <div class="flex-1 overflow-auto">
            <table class="w-full border-collapse text-sm">
              <thead class="sticky top-0 z-10 bg-slate-900">
                <tr>
                  <th class="px-4 py-2.5 border-b border-slate-800 w-10">
                    <input
                      type="checkbox"
                      :checked="allSelected"
                      @change="toggleSelectAll"
                      class="accent-violet-500 cursor-pointer"
                    />
                  </th>
                  <th class="px-3 py-2.5 text-left border-b border-r border-slate-800 text-xs font-medium text-slate-400 min-w-[280px]">Name</th>
                  <th class="px-4 py-2.5 text-left border-b border-r border-slate-800 text-xs font-medium text-slate-400 w-36">MIME Type</th>
                  <th class="px-4 py-2.5 text-left border-b border-r border-slate-800 text-xs font-medium text-slate-400 w-24">Size</th>
                  <th class="px-4 py-2.5 text-left border-b border-r border-slate-800 text-xs font-medium text-slate-400 w-40">Last Modified</th>
                  <th class="px-4 py-2.5 border-b border-slate-800 w-10"></th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="file in filteredFiles"
                  :key="file.id"
                  class="border-b border-slate-800/50 hover:bg-slate-800/30 transition-colors group"
                  :class="selectedFileIds.has(file.id) ? 'bg-violet-600/5' : ''"
                >
                  <td class="px-4 py-2.5 text-center">
                    <input
                      type="checkbox"
                      :checked="selectedFileIds.has(file.id)"
                      @change="toggleFile(file.id)"
                      class="accent-violet-500 cursor-pointer"
                    />
                  </td>
                  <td class="px-3 py-2.5 border-r border-slate-800/50">
                    <div class="flex items-center gap-2.5">
                      <!-- Icon -->
                      <div class="size-7 rounded flex items-center justify-center shrink-0 bg-slate-800">
                        <svg v-if="fileIcon(file.type) === 'image'" class="size-4 text-sky-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75">
                          <rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5"/><polyline points="21 15 16 10 5 21"/>
                        </svg>
                        <svg v-else-if="fileIcon(file.type) === 'video'" class="size-4 text-violet-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75">
                          <polygon points="23 7 16 12 23 17 23 7"/><rect x="1" y="5" width="15" height="14" rx="2"/>
                        </svg>
                        <svg v-else-if="fileIcon(file.type) === 'pdf'" class="size-4 text-rose-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75">
                          <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/><polyline points="14 2 14 8 20 8"/>
                        </svg>
                        <svg v-else-if="fileIcon(file.type) === 'text'" class="size-4 text-slate-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75">
                          <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/>
                        </svg>
                        <svg v-else class="size-4 text-slate-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75">
                          <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/><polyline points="14 2 14 8 20 8"/>
                        </svg>
                      </div>
                      <div class="min-w-0">
                        <p class="text-xs font-medium text-slate-200 truncate">{{ file.name }}</p>
                        <p class="text-[10px] text-slate-600 font-mono truncate">{{ file.path }}</p>
                      </div>
                    </div>
                  </td>
                  <td class="px-4 py-2.5 border-r border-slate-800/50 text-xs text-slate-500 font-mono truncate max-w-0">
                    <span :title="file.type">{{ file.type }}</span>
                  </td>
                  <td class="px-4 py-2.5 border-r border-slate-800/50 text-xs text-slate-400 font-mono whitespace-nowrap">
                    {{ formatSize(file.size) }}
                  </td>
                  <td class="px-4 py-2.5 border-r border-slate-800/50 text-xs text-slate-500 whitespace-nowrap">
                    {{ file.lastModified }}
                  </td>
                  <td class="px-3 py-2.5 text-center">
                    <button
                      @click="confirmDeleteFile = file.id"
                      class="opacity-0 group-hover:opacity-100 size-6 flex items-center justify-center text-slate-700 hover:text-rose-400 hover:bg-rose-500/10 rounded transition-all mx-auto"
                      title="Delete file"
                    >
                      <svg class="size-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <polyline points="3 6 5 6 21 6"/>
                        <path d="M19 6l-1 14a2 2 0 01-2 2H8a2 2 0 01-2-2L5 6"/>
                      </svg>
                    </button>
                  </td>
                </tr>

                <tr v-if="filteredFiles.length === 0">
                  <td colspan="6" class="py-20 text-center">
                    <div class="flex flex-col items-center gap-2">
                      <svg class="size-8 text-slate-700" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25">
                        <ellipse cx="12" cy="5" rx="9" ry="3"/>
                        <path d="M21 12c0 1.66-4.03 3-9 3S3 13.66 3 12"/>
                        <path d="M3 5v14c0 1.66 4.03 3 9 3s9-1.34 9-3V5"/>
                      </svg>
                      <p class="text-sm text-slate-600">{{ fileSearch ? 'No files match your search' : 'This bucket is empty' }}</p>
                      <p v-if="!fileSearch" class="text-xs text-slate-700">Drag & drop files here or use the Upload button</p>
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </template>

      <!-- No buckets -->
      <div v-else class="flex-1 flex flex-col items-center justify-center text-center p-8">
        <div class="size-16 rounded-2xl bg-slate-800/80 border border-slate-700/50 flex items-center justify-center mb-4">
          <svg class="size-7 text-slate-600" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <ellipse cx="12" cy="5" rx="9" ry="3"/>
            <path d="M21 12c0 1.66-4.03 3-9 3S3 13.66 3 12"/>
            <path d="M3 5v14c0 1.66 4.03 3 9 3s9-1.34 9-3V5"/>
          </svg>
        </div>
        <h2 class="text-base font-semibold text-slate-300 mb-1">No buckets yet</h2>
        <p class="text-sm text-slate-600 mb-5">Create a bucket to start storing files.</p>
        <button
          @click="showNewBucketModal = true"
          class="flex items-center gap-2 px-4 py-2 bg-violet-600 hover:bg-violet-500 text-white text-sm font-medium rounded-lg transition-colors"
        >
          <svg class="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>
          </svg>
          New Bucket
        </button>
      </div>
    </main>

    <!-- New Bucket Modal -->
    <Teleport to="body">
      <div
        v-if="showNewBucketModal"
        class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm"
        @click.self="showNewBucketModal = false"
      >
        <div class="bg-slate-900 rounded-xl border border-slate-700 w-full max-w-sm flex flex-col shadow-2xl">
          <div class="flex items-center justify-between px-6 py-4 border-b border-slate-800">
            <h2 class="text-base font-semibold text-slate-100">New Bucket</h2>
            <button @click="showNewBucketModal = false" class="text-slate-500 hover:text-slate-300 transition-colors">
              <svg class="size-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
              </svg>
            </button>
          </div>

          <div class="px-6 py-5 flex flex-col gap-4">
            <div>
              <label class="block text-xs font-medium text-slate-400 mb-1.5">Bucket Name</label>
              <input
                v-model="newBucketName"
                type="text"
                placeholder="e.g. user-uploads"
                @keyup.enter="addBucket"
                class="w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-100 placeholder-slate-600 focus:outline-none focus:ring-1 focus:ring-violet-500 focus:border-violet-500"
              />
              <p class="text-[10px] text-slate-600 mt-1">Lowercase letters, numbers, and hyphens only</p>
            </div>

            <div class="flex items-center justify-between bg-slate-800/50 border border-slate-700/50 rounded-lg px-4 py-3">
              <div>
                <p class="text-sm font-medium text-slate-300">Public bucket</p>
                <p class="text-xs text-slate-600 mt-0.5">Files accessible without authentication</p>
              </div>
              <button
                @click="newBucketPublic = !newBucketPublic"
                class="relative w-11 h-6 rounded-full transition-colors shrink-0 cursor-pointer"
                :class="newBucketPublic ? 'bg-violet-600' : 'bg-slate-700'"
              >
                <span
                  class="absolute top-0.5 size-5 rounded-full bg-white shadow transition-all duration-200"
                  :class="newBucketPublic ? 'left-[22px]' : 'left-0.5'"
                ></span>
              </button>
            </div>

            <div v-if="newBucketError" class="flex items-center gap-2 bg-rose-950/50 border border-rose-800 rounded-lg px-3 py-2">
              <svg class="size-3.5 text-rose-400 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/>
              </svg>
              <span class="text-xs text-rose-300">{{ newBucketError }}</span>
            </div>
          </div>

          <div class="flex items-center justify-end gap-2.5 px-6 py-4 border-t border-slate-800">
            <button @click="showNewBucketModal = false" class="px-4 py-2 text-sm text-slate-400 hover:text-slate-200 hover:bg-slate-800 rounded-lg transition-colors">
              Cancel
            </button>
            <button @click="addBucket" class="px-4 py-2 text-sm font-medium bg-violet-600 hover:bg-violet-500 text-white rounded-lg transition-colors">
              Create Bucket
            </button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Confirm delete bucket -->
    <Teleport to="body">
      <div
        v-if="confirmDeleteBucket"
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm"
      >
        <div class="bg-slate-900 border border-slate-700 rounded-xl p-6 w-full max-w-sm shadow-2xl">
          <div class="flex items-start gap-3 mb-4">
            <div class="size-9 rounded-full bg-rose-500/10 border border-rose-500/20 flex items-center justify-center shrink-0">
              <svg class="size-4.5 text-rose-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/>
                <line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/>
              </svg>
            </div>
            <div>
              <h3 class="text-sm font-semibold text-slate-100">Delete bucket?</h3>
              <p class="text-xs text-slate-500 mt-1">All files inside <span class="text-slate-300 font-medium">{{ buckets.find(b => b.id === confirmDeleteBucket)?.name }}</span> will be permanently deleted.</p>
            </div>
          </div>
          <div class="flex gap-2 justify-end">
            <button @click="confirmDeleteBucket = null" class="px-3 py-1.5 text-sm text-slate-400 hover:text-slate-200 hover:bg-slate-800 rounded-md transition-colors">Cancel</button>
            <button @click="deleteBucket(confirmDeleteBucket!)" class="px-3 py-1.5 text-sm font-medium bg-rose-600 hover:bg-rose-500 text-white rounded-md transition-colors">Delete</button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Confirm delete file -->
    <Teleport to="body">
      <div
        v-if="confirmDeleteFile"
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm"
      >
        <div class="bg-slate-900 border border-slate-700 rounded-xl p-6 w-full max-w-sm shadow-2xl">
          <div class="flex items-start gap-3 mb-4">
            <div class="size-9 rounded-full bg-rose-500/10 border border-rose-500/20 flex items-center justify-center shrink-0">
              <svg class="size-4.5 text-rose-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/>
                <line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/>
              </svg>
            </div>
            <div>
              <h3 class="text-sm font-semibold text-slate-100">Delete file?</h3>
              <p class="text-xs text-slate-500 mt-1">
                <span class="text-slate-300 font-medium font-mono">{{ activeBucket?.files.find(f => f.id === confirmDeleteFile)?.name }}</span> will be permanently deleted.
              </p>
            </div>
          </div>
          <div class="flex gap-2 justify-end">
            <button @click="confirmDeleteFile = null" class="px-3 py-1.5 text-sm text-slate-400 hover:text-slate-200 hover:bg-slate-800 rounded-md transition-colors">Cancel</button>
            <button @click="deleteFile(confirmDeleteFile!)" class="px-3 py-1.5 text-sm font-medium bg-rose-600 hover:bg-rose-500 text-white rounded-md transition-colors">Delete</button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>
