// Thin $fetch wrapper pre-configured with the scylladb-meta base URL.
// All composables import from here so the base URL is defined once.

export function useApi() {
  const config = useRuntimeConfig()
  const base = config.public.apiBase as string

  function get<T>(path: string) {
    return $fetch<T>(`${base}${path}`)
  }

  function post<T>(path: string, body: Record<string, unknown>) {
    return $fetch<T>(`${base}${path}`, { method: 'POST', body })
  }

  function patch<T>(path: string, body: Record<string, unknown>) {
    return $fetch<T>(`${base}${path}`, { method: 'PATCH', body })
  }

  function del<T>(path: string, body?: Record<string, unknown>) {
    return $fetch<T>(`${base}${path}`, { method: 'DELETE', ...(body ? { body } : {}) })
  }

  return { get, post, patch, del }
}
