<script setup lang="ts">
import { type Bucket, type Folder, type StorageFile } from '~/composables/useStorageApi'

const {
  listBuckets,
  createBucket,
  deleteBucket,
  listFolders,
  createFolder,
  deleteFolder,
  listFiles,
  createFile,
  deleteFile,
} = useStorageApi()

// ── State ──────────────────────────────────────────────────────────────────────

const buckets = ref<Bucket[]>([])
const folders = ref<Folder[]>([])
const files = ref<StorageFile[]>([])
const loading = ref(false)
const apiError = ref('')

const activeBucketId = ref('')
const currentFolderId = ref<string | undefined>(undefined) // undefined = root
const breadcrumbs = ref<{ id: string; name: string }[]>([])

const fileSearch = ref('')
const selectedFileIds = ref<Set<string>>(new Set())
const isDraggingOver = ref(false)

// ── Modals ─────────────────────────────────────────────────────────────────────

const showNewBucketModal = ref(false)
const newBucketName = ref('')
const newBucketPublic = ref(false)
const newBucketError = ref('')
const newBucketLoading = ref(false)

const showNewFolderModal = ref(false)
const newFolderName = ref('')
const newFolderError = ref('')
const newFolderLoading = ref(false)

const confirmDeleteBucket = ref<string | null>(null)
const confirmDeleteFolder = ref<Folder | null>(null)
const confirmDeleteFile = ref<StorageFile | null>(null)
const deleteLoading = ref(false)

// ── Computed ───────────────────────────────────────────────────────────────────

const activeBucket = computed(() => buckets.value.find(b => b.id === activeBucketId.value))

const filteredFiles = computed(() =>
  files.value.filter(f => f.name.toLowerCase().includes(fileSearch.value.toLowerCase())),
)

const allSelected = computed(() =>
  filteredFiles.value.length > 0 &&
  filteredFiles.value.every(f => selectedFileIds.value.has(f.id)),
)

// ── Data loading ───────────────────────────────────────────────────────────────

async function loadBuckets() {
  loading.value = true
  apiError.value = ''
  try {
    buckets.value = await listBuckets()
    if (buckets.value.length > 0 && !activeBucketId.value) {
      activeBucketId.value = buckets.value[0].id
    }
  } catch (e: unknown) {
    apiError.value = String(e)
  } finally {
    loading.value = false
  }
}

async function loadContent() {
  const bid = activeBucketId.value
  if (!bid) return
  loading.value = true
  try {
    const [f, fl] = await Promise.all([
      listFolders(bid, currentFolderId.value),
      listFiles(bid, currentFolderId.value),
    ])
    folders.value = f
    files.value = fl
    selectedFileIds.value.clear()
  } catch (e: unknown) {
    apiError.value = String(e)
  } finally {
    loading.value = false
  }
}

onMounted(loadBuckets)

watch(activeBucketId, () => {
  currentFolderId.value = undefined
  breadcrumbs.value = []
  folders.value = []
  files.value = []
  if (activeBucketId.value) loadContent()
})

watch(currentFolderId, () => {
  if (activeBucketId.value) loadContent()
})

// ── Navigation ─────────────────────────────────────────────────────────────────

function navigateInto(folder: Folder) {
  breadcrumbs.value.push({ id: folder.id, name: folder.name })
  currentFolderId.value = folder.id
}

function navigateTo(index: number) {
  if (index === -1) {
    breadcrumbs.value = []
    currentFolderId.value = undefined
  } else {
    breadcrumbs.value = breadcrumbs.value.slice(0, index + 1)
    currentFolderId.value = breadcrumbs.value[index].id
  }
}

// ── Bucket CRUD ────────────────────────────────────────────────────────────────

async function addBucket() {
  newBucketError.value = ''
  const name = newBucketName.value.trim().toLowerCase().replace(/\s+/g, '-')
  if (!name) { newBucketError.value = 'Bucket name is required'; return }
  if (!/^[a-z0-9-]+$/.test(name)) { newBucketError.value = 'Only lowercase letters, numbers, and hyphens allowed'; return }
  if (buckets.value.find(b => b.name === name)) { newBucketError.value = 'Bucket name already exists'; return }

  newBucketLoading.value = true
  try {
    const bucket = await createBucket(name, newBucketPublic.value)
    buckets.value.push(bucket)
    activeBucketId.value = bucket.id
    newBucketName.value = ''
    newBucketPublic.value = false
    showNewBucketModal.value = false
  } catch (e: unknown) {
    newBucketError.value = String(e)
  } finally {
    newBucketLoading.value = false
  }
}

async function removeBucket(id: string) {
  deleteLoading.value = true
  try {
    await deleteBucket(id)
    buckets.value = buckets.value.filter(b => b.id !== id)
    if (activeBucketId.value === id)
      activeBucketId.value = buckets.value[0]?.id ?? ''
    confirmDeleteBucket.value = null
  } catch (e: unknown) {
    apiError.value = String(e)
  } finally {
    deleteLoading.value = false
  }
}

// ── Folder CRUD ────────────────────────────────────────────────────────────────

async function addFolder() {
  newFolderError.value = ''
  const name = newFolderName.value.trim()
  if (!name) { newFolderError.value = 'Folder name is required'; return }

  newFolderLoading.value = true
  try {
    const folder = await createFolder(activeBucketId.value, name, currentFolderId.value)
    folders.value.push(folder)
    newFolderName.value = ''
    showNewFolderModal.value = false
  } catch (e: unknown) {
    newFolderError.value = String(e)
  } finally {
    newFolderLoading.value = false
  }
}

async function removeFolder(folder: Folder) {
  deleteLoading.value = true
  try {
    await deleteFolder(activeBucketId.value, folder.id)
    folders.value = folders.value.filter(f => f.id !== folder.id)
    confirmDeleteFolder.value = null
  } catch (e: unknown) {
    apiError.value = String(e)
  } finally {
    deleteLoading.value = false
  }
}

// ── File CRUD ──────────────────────────────────────────────────────────────────

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

async function uploadFiles(names: string[]) {
  if (!activeBucketId.value) return
  for (const name of names) {
    try {
      const file = await createFile(activeBucketId.value, {
        name,
        size: Math.floor(Math.random() * 500000) + 5000,
        content_type: guessType(name),
        folder_id: currentFolderId.value,
      })
      files.value.push(file)
    } catch (e: unknown) {
      apiError.value = String(e)
    }
  }
}

async function removeFile(file: StorageFile) {
  deleteLoading.value = true
  try {
    await deleteFile(activeBucketId.value, file.id)
    files.value = files.value.filter(f => f.id !== file.id)
    selectedFileIds.value.delete(file.id)
    confirmDeleteFile.value = null
  } catch (e: unknown) {
    apiError.value = String(e)
  } finally {
    deleteLoading.value = false
  }
}

async function deleteSelected() {
  const ids = [...selectedFileIds.value]
  for (const id of ids) {
    const file = files.value.find(f => f.id === id)
    if (!file) continue
    try {
      await deleteFile(activeBucketId.value, id)
      files.value = files.value.filter(f => f.id !== id)
      selectedFileIds.value.delete(id)
    } catch { /* continue */ }
  }
}

// ── Utilities ──────────────────────────────────────────────────────────────────

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
  const dropped = Array.from(e.dataTransfer?.files ?? [])
  if (dropped.length) uploadFiles(dropped.map(f => f.name))
}

function onFileInputChange(e: Event) {
  const picked = Array.from((e.target as HTMLInputElement).files ?? [])
  if (picked.length) uploadFiles(picked.map(f => f.name))
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
        <div v-if="loading && buckets.length === 0" class="px-3 py-4 text-center">
          <span class="text-xs text-slate-600">Loading...</span>
        </div>
        <button
          v-for="bucket in buckets"
          :key="bucket.id"
          @click="activeBucketId = bucket.id; fileSearch = ''"
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
          </div>
        </button>
      </nav>
    </aside>

    <!-- Main content -->
    <main class="flex-1 flex flex-col min-w-0 overflow-hidden">
      <!-- API error banner -->
      <div
        v-if="apiError"
        class="shrink-0 flex items-center gap-2 px-4 py-2 bg-rose-950/50 border-b border-rose-800 text-xs text-rose-300"
      >
        <svg class="size-3.5 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/>
        </svg>
        {{ apiError }}
        <button @click="apiError = ''" class="ml-auto text-rose-500 hover:text-rose-300">x</button>
      </div>

      <template v-if="activeBucket">
        <!-- Toolbar -->
        <header class="flex items-center gap-3 px-5 py-3 border-b border-slate-800 bg-slate-900/30 shrink-0">
          <div class="flex items-center gap-2 min-w-0 flex-1">
            <span class="text-sm font-semibold text-slate-100 truncate">{{ activeBucket.name }}</span>
            <span
              class="text-xs px-1.5 py-0.5 rounded font-semibold shrink-0"
              :class="activeBucket.public ? 'bg-emerald-500/15 text-emerald-400' : 'bg-slate-700 text-slate-500'"
            >{{ activeBucket.public ? 'Public' : 'Private' }}</span>
            <span class="text-xs text-slate-600 shrink-0">{{ folders.length }} folders / {{ files.length }} files</span>
          </div>

          <!-- Search -->
          <div class="relative w-44 shrink-0">
            <svg class="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-slate-600 pointer-events-none" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
            </svg>
            <input
              v-model="fileSearch"
              type="text"
              placeholder="Search files..."
              class="w-full bg-slate-800/80 border border-slate-700/50 rounded-md pl-7 pr-2.5 py-1.5 text-xs text-slate-300 placeholder-slate-600 focus:outline-none focus:border-violet-500/50 focus:ring-1 focus:ring-violet-500/30"
            />
          </div>

          <!-- Delete selected files -->
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

          <!-- New folder -->
          <button
            @click="showNewFolderModal = true"
            class="flex items-center gap-1.5 px-2.5 py-1.5 text-xs text-slate-300 hover:text-slate-100 hover:bg-slate-800 rounded-md border border-slate-700/50 transition-colors shrink-0"
          >
            <svg class="size-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z"/>
              <line x1="12" y1="11" x2="12" y2="17"/><line x1="9" y1="14" x2="15" y2="14"/>
            </svg>
            New Folder
          </button>

          <!-- Upload -->
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

        <!-- Breadcrumbs -->
        <div
          v-if="breadcrumbs.length > 0"
          class="flex items-center gap-1 px-5 py-2 border-b border-slate-800 bg-slate-900/20 shrink-0"
        >
          <button
            @click="navigateTo(-1)"
            class="text-xs text-slate-400 hover:text-violet-300 transition-colors"
          >{{ activeBucket.name }}</button>
          <template v-for="(crumb, i) in breadcrumbs" :key="crumb.id">
            <svg class="size-3 text-slate-700 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <polyline points="9 18 15 12 9 6"/>
            </svg>
            <button
              @click="navigateTo(i)"
              class="text-xs transition-colors"
              :class="i === breadcrumbs.length - 1 ? 'text-slate-200 font-medium' : 'text-slate-400 hover:text-violet-300'"
            >{{ crumb.name }}</button>
          </template>
        </div>

        <!-- Drop zone + listing -->
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

          <!-- Loading spinner -->
          <div v-if="loading" class="absolute inset-0 z-10 flex items-center justify-center bg-slate-950/40">
            <div class="size-5 border-2 border-violet-500 border-t-transparent rounded-full animate-spin"></div>
          </div>

          <!-- Table -->
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
                  <th class="px-3 py-2.5 text-left border-b border-r border-slate-800 text-xs font-medium text-slate-400 min-w-70">Name</th>
                  <th class="px-4 py-2.5 text-left border-b border-r border-slate-800 text-xs font-medium text-slate-400 w-36">MIME Type</th>
                  <th class="px-4 py-2.5 text-left border-b border-r border-slate-800 text-xs font-medium text-slate-400 w-24">Size</th>
                  <th class="px-4 py-2.5 text-left border-b border-r border-slate-800 text-xs font-medium text-slate-400 w-40">Created</th>
                  <th class="px-4 py-2.5 border-b border-slate-800 w-10"></th>
                </tr>
              </thead>
              <tbody>
                <!-- Folder rows -->
                <tr
                  v-for="folder in folders"
                  :key="folder.id"
                  class="border-b border-slate-800/50 hover:bg-slate-800/30 transition-colors group cursor-pointer"
                  @click="navigateInto(folder)"
                >
                  <td class="px-4 py-2.5 text-center"></td>
                  <td class="px-3 py-2.5 border-r border-slate-800/50">
                    <div class="flex items-center gap-2.5">
                      <div class="size-7 rounded flex items-center justify-center shrink-0 bg-amber-500/10">
                        <svg class="size-4 text-amber-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75">
                          <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z"/>
                        </svg>
                      </div>
                      <span class="text-xs font-medium text-slate-200">{{ folder.name }}</span>
                    </div>
                  </td>
                  <td class="px-4 py-2.5 border-r border-slate-800/50 text-xs text-slate-600">folder</td>
                  <td class="px-4 py-2.5 border-r border-slate-800/50 text-xs text-slate-600">—</td>
                  <td class="px-4 py-2.5 border-r border-slate-800/50 text-xs text-slate-500 whitespace-nowrap">{{ folder.created_at.slice(0, 10) }}</td>
                  <td class="px-3 py-2.5 text-center" @click.stop>
                    <button
                      @click="confirmDeleteFolder = folder"
                      class="opacity-0 group-hover:opacity-100 size-6 flex items-center justify-center text-slate-700 hover:text-rose-400 hover:bg-rose-500/10 rounded transition-all mx-auto"
                    >
                      <svg class="size-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <polyline points="3 6 5 6 21 6"/>
                        <path d="M19 6l-1 14a2 2 0 01-2 2H8a2 2 0 01-2-2L5 6"/>
                      </svg>
                    </button>
                  </td>
                </tr>

                <!-- File rows -->
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
                      <div class="size-7 rounded flex items-center justify-center shrink-0 bg-slate-800">
                        <svg v-if="fileIcon(file.content_type) === 'image'" class="size-4 text-sky-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75">
                          <rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5"/><polyline points="21 15 16 10 5 21"/>
                        </svg>
                        <svg v-else-if="fileIcon(file.content_type) === 'video'" class="size-4 text-violet-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75">
                          <polygon points="23 7 16 12 23 17 23 7"/><rect x="1" y="5" width="15" height="14" rx="2"/>
                        </svg>
                        <svg v-else-if="fileIcon(file.content_type) === 'pdf'" class="size-4 text-rose-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75">
                          <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/><polyline points="14 2 14 8 20 8"/>
                        </svg>
                        <svg v-else-if="fileIcon(file.content_type) === 'text'" class="size-4 text-slate-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75">
                          <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/>
                        </svg>
                        <svg v-else class="size-4 text-slate-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75">
                          <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/><polyline points="14 2 14 8 20 8"/>
                        </svg>
                      </div>
                      <div class="min-w-0">
                        <p class="text-xs font-medium text-slate-200 truncate">{{ file.name }}</p>
                        <p class="text-[10px] text-slate-600 font-mono truncate">{{ file.storage_path }}</p>
                      </div>
                    </div>
                  </td>
                  <td class="px-4 py-2.5 border-r border-slate-800/50 text-xs text-slate-500 font-mono truncate max-w-0">
                    <span :title="file.content_type">{{ file.content_type || '—' }}</span>
                  </td>
                  <td class="px-4 py-2.5 border-r border-slate-800/50 text-xs text-slate-400 font-mono whitespace-nowrap">
                    {{ file.size ? formatSize(file.size) : '—' }}
                  </td>
                  <td class="px-4 py-2.5 border-r border-slate-800/50 text-xs text-slate-500 whitespace-nowrap">
                    {{ file.created_at.slice(0, 10) }}
                  </td>
                  <td class="px-3 py-2.5 text-center">
                    <button
                      @click="confirmDeleteFile = file"
                      class="opacity-0 group-hover:opacity-100 size-6 flex items-center justify-center text-slate-700 hover:text-rose-400 hover:bg-rose-500/10 rounded transition-all mx-auto"
                    >
                      <svg class="size-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <polyline points="3 6 5 6 21 6"/>
                        <path d="M19 6l-1 14a2 2 0 01-2 2H8a2 2 0 01-2-2L5 6"/>
                      </svg>
                    </button>
                  </td>
                </tr>

                <!-- Empty state -->
                <tr v-if="!loading && folders.length === 0 && filteredFiles.length === 0">
                  <td colspan="6" class="py-20 text-center">
                    <div class="flex flex-col items-center gap-2">
                      <svg class="size-8 text-slate-700" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25">
                        <ellipse cx="12" cy="5" rx="9" ry="3"/>
                        <path d="M21 12c0 1.66-4.03 3-9 3S3 13.66 3 12"/>
                        <path d="M3 5v14c0 1.66 4.03 3 9 3s9-1.34 9-3V5"/>
                      </svg>
                      <p class="text-sm text-slate-600">{{ fileSearch ? 'No files match your search' : 'This folder is empty' }}</p>
                      <p v-if="!fileSearch" class="text-xs text-slate-700">Drag and drop files or use New Folder / Upload</p>
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

    <!-- ── Modals ────────────────────────────────────────────────────────────── -->

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
                <span class="absolute top-0.5 size-5 rounded-full bg-white shadow transition-all duration-200" :class="newBucketPublic ? 'left-5.5' : 'left-0.5'"></span>
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
            <button @click="showNewBucketModal = false" class="px-4 py-2 text-sm text-slate-400 hover:text-slate-200 hover:bg-slate-800 rounded-lg transition-colors">Cancel</button>
            <button @click="addBucket" :disabled="newBucketLoading" class="px-4 py-2 text-sm font-medium bg-violet-600 hover:bg-violet-500 text-white rounded-lg transition-colors disabled:opacity-50">
              {{ newBucketLoading ? 'Creating...' : 'Create Bucket' }}
            </button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- New Folder Modal -->
    <Teleport to="body">
      <div
        v-if="showNewFolderModal"
        class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm"
        @click.self="showNewFolderModal = false"
      >
        <div class="bg-slate-900 rounded-xl border border-slate-700 w-full max-w-sm flex flex-col shadow-2xl">
          <div class="flex items-center justify-between px-6 py-4 border-b border-slate-800">
            <h2 class="text-base font-semibold text-slate-100">New Folder</h2>
            <button @click="showNewFolderModal = false" class="text-slate-500 hover:text-slate-300 transition-colors">
              <svg class="size-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
              </svg>
            </button>
          </div>
          <div class="px-6 py-5">
            <label class="block text-xs font-medium text-slate-400 mb-1.5">Folder Name</label>
            <input
              v-model="newFolderName"
              type="text"
              placeholder="e.g. 2024-uploads"
              @keyup.enter="addFolder"
              class="w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-100 placeholder-slate-600 focus:outline-none focus:ring-1 focus:ring-violet-500 focus:border-violet-500"
            />
            <div v-if="newFolderError" class="flex items-center gap-2 bg-rose-950/50 border border-rose-800 rounded-lg px-3 py-2 mt-3">
              <svg class="size-3.5 text-rose-400 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/>
              </svg>
              <span class="text-xs text-rose-300">{{ newFolderError }}</span>
            </div>
          </div>
          <div class="flex items-center justify-end gap-2.5 px-6 py-4 border-t border-slate-800">
            <button @click="showNewFolderModal = false" class="px-4 py-2 text-sm text-slate-400 hover:text-slate-200 hover:bg-slate-800 rounded-lg transition-colors">Cancel</button>
            <button @click="addFolder" :disabled="newFolderLoading" class="px-4 py-2 text-sm font-medium bg-violet-600 hover:bg-violet-500 text-white rounded-lg transition-colors disabled:opacity-50">
              {{ newFolderLoading ? 'Creating...' : 'Create Folder' }}
            </button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Confirm delete bucket -->
    <Teleport to="body">
      <div v-if="confirmDeleteBucket" class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm">
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
              <p class="text-xs text-slate-500 mt-1">All files and folders inside <span class="text-slate-300 font-medium">{{ buckets.find(b => b.id === confirmDeleteBucket)?.name }}</span> will be permanently deleted.</p>
            </div>
          </div>
          <div class="flex gap-2 justify-end">
            <button @click="confirmDeleteBucket = null" class="px-3 py-1.5 text-sm text-slate-400 hover:text-slate-200 hover:bg-slate-800 rounded-md transition-colors">Cancel</button>
            <button @click="removeBucket(confirmDeleteBucket!)" :disabled="deleteLoading" class="px-3 py-1.5 text-sm font-medium bg-rose-600 hover:bg-rose-500 text-white rounded-md transition-colors disabled:opacity-50">
              {{ deleteLoading ? 'Deleting...' : 'Delete' }}
            </button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Confirm delete folder -->
    <Teleport to="body">
      <div v-if="confirmDeleteFolder" class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm">
        <div class="bg-slate-900 border border-slate-700 rounded-xl p-6 w-full max-w-sm shadow-2xl">
          <div class="flex items-start gap-3 mb-4">
            <div class="size-9 rounded-full bg-rose-500/10 border border-rose-500/20 flex items-center justify-center shrink-0">
              <svg class="size-4.5 text-rose-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/>
                <line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/>
              </svg>
            </div>
            <div>
              <h3 class="text-sm font-semibold text-slate-100">Delete folder?</h3>
              <p class="text-xs text-slate-500 mt-1"><span class="text-slate-300 font-medium font-mono">{{ confirmDeleteFolder.name }}</span> will be permanently deleted.</p>
            </div>
          </div>
          <div class="flex gap-2 justify-end">
            <button @click="confirmDeleteFolder = null" class="px-3 py-1.5 text-sm text-slate-400 hover:text-slate-200 hover:bg-slate-800 rounded-md transition-colors">Cancel</button>
            <button @click="removeFolder(confirmDeleteFolder!)" :disabled="deleteLoading" class="px-3 py-1.5 text-sm font-medium bg-rose-600 hover:bg-rose-500 text-white rounded-md transition-colors disabled:opacity-50">
              {{ deleteLoading ? 'Deleting...' : 'Delete' }}
            </button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Confirm delete file -->
    <Teleport to="body">
      <div v-if="confirmDeleteFile" class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm">
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
              <p class="text-xs text-slate-500 mt-1"><span class="text-slate-300 font-medium font-mono">{{ confirmDeleteFile.name }}</span> will be permanently deleted.</p>
            </div>
          </div>
          <div class="flex gap-2 justify-end">
            <button @click="confirmDeleteFile = null" class="px-3 py-1.5 text-sm text-slate-400 hover:text-slate-200 hover:bg-slate-800 rounded-md transition-colors">Cancel</button>
            <button @click="removeFile(confirmDeleteFile!)" :disabled="deleteLoading" class="px-3 py-1.5 text-sm font-medium bg-rose-600 hover:bg-rose-500 text-white rounded-md transition-colors disabled:opacity-50">
              {{ deleteLoading ? 'Deleting...' : 'Delete' }}
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>
