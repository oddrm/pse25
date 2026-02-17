// frontend/stores/logsStore.ts
import { defineStore, skipHydrate } from 'pinia'
import { useLocalStorage } from '@vueuse/core'
import { ref } from 'vue'

export interface LogEntry {
  timestamp: string
  level: string
  message: string
  location?: string
}

export const useLogsStore = defineStore('logs', () => {
  const logs = ref<LogEntry[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  // Use persistent storage for preferences
  const levelFilter = useLocalStorage('logs-level-filter', 'DEBUG')
  const limit = useLocalStorage('logs-limit', 200)

  async function fetchLogs() {
    loading.value = true
    try {
      const url = `/backend/logs?limit=${limit.value}&level=${levelFilter.value}`
      const response = await fetch(url)
      if (!response.ok) throw new Error('Failed to fetch logs')
      logs.value = await response.json()
    } catch (err: any) {
      error.value = err.message
    } finally {
      loading.value = false
    }
  }

  function setLevelFilter(level: string) {
    levelFilter.value = level
    fetchLogs()
  }

  function setLimit(newLimit: number) {
    limit.value = newLimit
    fetchLogs()
  }

  return {
    logs,
    loading,
    error,
    levelFilter: skipHydrate(levelFilter),
    limit: skipHydrate(limit),
    fetchLogs,
    setLevelFilter,
    setLimit,
  }
})
