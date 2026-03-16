import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { ref } from 'vue'
import { useLogsStore } from './logsStore'

const localStorageMock = vi.fn()

vi.mock('@vueuse/core', () => ({
  useLocalStorage: (...args: unknown[]) => localStorageMock(...args),
}))

describe('logsStore', () => {
  const fetchMock = vi.fn()

  beforeEach(() => {
    setActivePinia(createPinia())
    fetchMock.mockReset()
    vi.stubGlobal('fetch', fetchMock)
    localStorageMock.mockReset()
    localStorageMock.mockImplementation((_key: string, initial: unknown) => ref(initial))
  })

  it('fetchLogs loads logs and clears loading state', async () => {
    const store = useLogsStore()
    fetchMock.mockResolvedValue({
      ok: true,
      json: vi.fn().mockResolvedValue([{ timestamp: 't', level: 'INFO', message: 'ok' }]),
    })

    await store.fetchLogs()

    expect(fetchMock).toHaveBeenCalledWith('/backend/logs?limit=200&level=DEBUG')
    expect(store.logs).toEqual([{ timestamp: 't', level: 'INFO', message: 'ok' }])
    expect(store.loading).toBe(false)
    expect(store.error).toBeNull()
  })

  it('fetchLogs stores backend failures', async () => {
    const store = useLogsStore()
    fetchMock.mockResolvedValue({ ok: false })

    await store.fetchLogs()

    expect(store.error).toBe('Failed to fetch logs')
    expect(store.loading).toBe(false)
  })

  it('fetchLogs stores thrown errors', async () => {
    const store = useLogsStore()
    fetchMock.mockRejectedValue(new Error('network'))

    await store.fetchLogs()

    expect(store.error).toBe('network')
  })

  it('setLevelFilter persists the level and triggers a refresh', async () => {
    const store = useLogsStore()
    fetchMock.mockResolvedValue({
      ok: true,
      json: vi.fn().mockResolvedValue([]),
    })

    await store.setLevelFilter('WARN')

    expect(store.levelFilter).toBe('WARN')
    expect(fetchMock).toHaveBeenCalledWith('/backend/logs?limit=200&level=WARN')
  })

  it('setLimit persists the limit and triggers a refresh', async () => {
    const store = useLogsStore()
    fetchMock.mockResolvedValue({
      ok: true,
      json: vi.fn().mockResolvedValue([]),
    })

    await store.setLimit(50)

    expect(store.limit).toBe(50)
    expect(fetchMock).toHaveBeenCalledWith('/backend/logs?limit=50&level=DEBUG')
  })
})
