// Types

export interface Bucket {
  id: string
  name: string
  public: boolean
  created_at: string
  updated_at: string
}

export interface Folder {
  id: string
  bucket_id: string
  parent_id: string
  name: string
  created_at: string
  updated_at: string
}

export interface StorageFile {
  id: string
  bucket_id: string
  folder_id: string
  name: string
  size: number
  content_type: string
  storage_path: string
  created_at: string
  updated_at: string
}

export interface CreateFileInput {
  name: string
  size?: number
  content_type?: string
  folder_id?: string
  storage_path?: string
}

// The nil UUID sentinel represents the root level (no parent / no folder)
export const NIL_UUID = '00000000-0000-0000-0000-000000000000'

export function useStorageApi() {
  const config = useRuntimeConfig()
  const base = config.public.storageBase as string

  // ── Buckets ──────────────────────────────────────────────────────────────────

  const listBuckets = () =>
    $fetch<Bucket[]>(`${base}/storage/buckets`)

  const createBucket = (name: string, isPublic: boolean) =>
    $fetch<Bucket>(`${base}/storage/buckets`, {
      method: 'POST',
      body: { name, public: isPublic },
    })

  const deleteBucket = (id: string) =>
    $fetch<void>(`${base}/storage/buckets/${id}`, { method: 'DELETE' })

  // ── Folders ──────────────────────────────────────────────────────────────────

  /** List direct children of a folder. Omit parentId to list root-level folders. */
  const listFolders = (bucketId: string, parentId?: string) =>
    $fetch<Folder[]>(`${base}/storage/buckets/${bucketId}/folders`, {
      query: parentId ? { parent_id: parentId } : {},
    })

  const createFolder = (bucketId: string, name: string, parentId?: string) =>
    $fetch<Folder>(`${base}/storage/buckets/${bucketId}/folders`, {
      method: 'POST',
      body: { name, ...(parentId ? { parent_id: parentId } : {}) },
    })

  const deleteFolder = (bucketId: string, folderId: string) =>
    $fetch<void>(`${base}/storage/buckets/${bucketId}/folders/${folderId}`, {
      method: 'DELETE',
    })

  // ── Files ─────────────────────────────────────────────────────────────────────

  /** List files inside a folder. Omit folderId to list root-level files. */
  const listFiles = (bucketId: string, folderId?: string) =>
    $fetch<StorageFile[]>(`${base}/storage/buckets/${bucketId}/files`, {
      query: folderId ? { folder_id: folderId } : {},
    })

  const createFile = (bucketId: string, input: CreateFileInput) =>
    $fetch<StorageFile>(`${base}/storage/buckets/${bucketId}/files`, {
      method: 'POST',
      body: input,
    })

  const deleteFile = (bucketId: string, fileId: string) =>
    $fetch<void>(`${base}/storage/buckets/${bucketId}/files/${fileId}`, {
      method: 'DELETE',
    })

  return {
    listBuckets,
    createBucket,
    deleteBucket,
    listFolders,
    createFolder,
    deleteFolder,
    listFiles,
    createFile,
    deleteFile,
  }
}
