import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { usePluginsStore } from './pluginStore'

const useLogsStoreMock = vi.fn(() => ({}))

vi.mock('./logsStore', () => ({
  useLogsStore: () => useLogsStoreMock(),
}))

describe('pluginStore', () => {
  const fetchMock = vi.fn()
  let intervalSpy: ReturnType<typeof vi.spyOn>
  let clearSpy: ReturnType<typeof vi.spyOn>

  beforeEach(() => {
    setActivePinia(createPinia())
    fetchMock.mockReset()
    vi.stubGlobal('fetch', fetchMock)
    intervalSpy = vi.spyOn(globalThis, 'setInterval')
    clearSpy = vi.spyOn(globalThis, 'clearInterval')
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('loadPlugins fetches plugins once and starts polling', async () => {
    const store = usePluginsStore()
    const pollingSpy = vi.spyOn(store, 'startPollingRunning').mockImplementation(() => {})
    fetchMock.mockResolvedValue({
      ok: true,
      json: vi.fn().mockResolvedValue([{ name: 'compress', description: 'Compress data' }]),
    })

    await store.loadPlugins()
    await store.loadPlugins()

    expect(fetchMock).toHaveBeenCalledTimes(1)
    expect(store.plugins).toEqual([
      {
        id: 1,
        name: 'compress',
        description: 'Compress data',
        isGlobalRunning: false,
        globalProgress: 0,
      },
    ])
    expect(pollingSpy).toHaveBeenCalledTimes(1)
  })

  it('loadPlugins keeps the list empty on backend failure', async () => {
    const store = usePluginsStore()
    fetchMock.mockResolvedValue({ ok: false })
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})

    await store.loadPlugins()

    expect(store.plugins).toEqual([])
    expect(errorSpy).toHaveBeenCalled()
  })

  it('startPollingRunning updates running state and plugin summaries', async () => {
    const store = usePluginsStore()
    store.plugins = [
      { id: 1, name: 'compress', description: 'Compress data', isGlobalRunning: false, globalProgress: 0 },
    ]
    store.runningPlugins = [
      { runId: 1, pluginName: 'compress', entryName: 'existing', progress: 20, state: 'Running' },
    ]
    fetchMock.mockResolvedValue({
      ok: true,
      json: vi.fn().mockResolvedValue([
        { instance_id: 2, name: 'compress', progress: 0.42, state: 'Running' },
        { instance_id: 1, name: 'compress', progress: 0.15, state: 'Paused' },
      ]),
    })

    store.startPollingRunning()

    expect(intervalSpy).toHaveBeenCalled()
    const callback = intervalSpy.mock.calls[0]?.[0] as TimerHandler
    await (callback as () => Promise<void>)()

    expect(store.runningPlugins).toEqual([
      { runId: '2', pluginName: 'compress', entryName: '', progress: 42, state: 'Running' },
      { runId: '1', pluginName: 'compress', entryName: 'existing', progress: 15, state: 'Paused' },
    ])
    expect(store.plugins[0]?.recentInstances).toHaveLength(2)
    expect(store.plugins[0]?.isGlobalRunning).toBe(true)
    expect(store.plugins[0]?.globalProgress).toBe(42)
  })

  it('startPollingRunning is idempotent and tolerates polling failures', async () => {
    const store = usePluginsStore()
    fetchMock.mockRejectedValue(new Error('nope'))

    store.startPollingRunning()
    const firstInterval = store._pollInterval
    store.startPollingRunning()

    expect(store._pollInterval).toBe(firstInterval)
    const callback = intervalSpy.mock.calls[0]?.[0] as TimerHandler
    await expect((callback as () => Promise<void>)()).resolves.toBeUndefined()
  })

  it('startPlugin skips missing plugins and duplicate entry launches', async () => {
    const store = usePluginsStore()
    store.plugins = [{ id: 1, name: 'compress', description: '', isGlobalRunning: false, globalProgress: 0 }]
    store.runningPlugins = [{ runId: 4, pluginName: 'compress', entryName: '/tmp/a', progress: 10, state: 'Running' }]

    await store.startPlugin(999, '/tmp/a')
    await store.startPlugin(1, '/tmp/a')

    expect(fetchMock).not.toHaveBeenCalled()
  })

  it('startPlugin starts entry-specific and global instances', async () => {
    const store = usePluginsStore()
    store.plugins = [{ id: 1, name: 'compress', description: '', isGlobalRunning: false, globalProgress: 0 }]
    fetchMock.mockResolvedValue({
      ok: true,
      status: 200,
      text: vi.fn().mockResolvedValue('17'),
    })

    await store.startPlugin(1, '/tmp/a', { mode: 'fast' })
    await store.startPlugin(1)

    expect(fetchMock).toHaveBeenNthCalledWith(1, '/backend/plugins/compress/start', {
      method: 'POST',
      body: JSON.stringify({ entry_path: '/tmp/a', payload: { mode: 'fast' } }),
    })
    expect(store.runningPlugins[0]).toEqual({
      runId: 17,
      pluginName: 'compress',
      entryName: '/tmp/a',
      progress: 0,
    })
    expect(store.plugins[0]?.isGlobalRunning).toBe(true)
    expect(store.runningPlugins[1]?.entryName).toBe('')
  })

  it('startPlugin swallows backend failures', async () => {
    const store = usePluginsStore()
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    store.plugins = [{ id: 1, name: 'compress', description: '', isGlobalRunning: false, globalProgress: 0 }]
    fetchMock.mockResolvedValue({
      ok: false,
      status: 500,
      text: vi.fn().mockResolvedValue('boom'),
    })

    await store.startPlugin(1, '/tmp/a')

    expect(store.runningPlugins).toEqual([])
    expect(errorSpy).toHaveBeenCalled()
  })

  it('registerPlugins resets the cache and reloads plugins', async () => {
    const store = usePluginsStore()
    const loadSpy = vi.spyOn(store, 'loadPlugins').mockResolvedValue()
    store.plugins = [{ id: 1, name: 'old', description: '', isGlobalRunning: false, globalProgress: 0 }]
    fetchMock.mockResolvedValue({ ok: true })

    await store.registerPlugins()

    expect(fetchMock).toHaveBeenCalledWith('/backend/plugins/register', { method: 'PUT' })
    expect(store.plugins).toEqual([])
    expect(loadSpy).toHaveBeenCalled()
  })

  it('registerPlugins logs failures', async () => {
    const store = usePluginsStore()
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    fetchMock.mockResolvedValue({ ok: false })

    await store.registerPlugins()

    expect(errorSpy).toHaveBeenCalled()
  })

  it('stop, pause, and resume update optimistic state', async () => {
    const store = usePluginsStore()
    store.runningPlugins = [{ runId: 21, pluginName: 'compress', entryName: '', progress: 10, state: 'Running' }]
    fetchMock.mockResolvedValue({ ok: true })

    await store.pauseInstance(21)
    await store.resumeInstance(21)
    await store.stopInstance(21)

    expect(store.runningPlugins[0]).toMatchObject({ state: 'Stopped', progress: 100 })
  })

  it('stop, pause, and resume log backend failures', async () => {
    const store = usePluginsStore()
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    fetchMock.mockResolvedValue({ ok: false })

    await store.stopInstance(1)
    await store.pauseInstance(1)
    await store.resumeInstance(1)

    expect(errorSpy).toHaveBeenCalledTimes(3)
  })

  it('stop, pause, and resume tolerate missing local instances', async () => {
    const store = usePluginsStore()
    fetchMock.mockResolvedValue({ ok: true })

    await store.stopInstance(1)
    await store.pauseInstance(1)
    await store.resumeInstance(1)

    expect(store.runningPlugins).toEqual([])
  })

  it('stopPolling clears the active interval', () => {
    const store = usePluginsStore()
    store._pollInterval = 123 as any

    store.stopPolling()

    expect(clearSpy).toHaveBeenCalledWith(123)
    expect(store._pollInterval).toBeNull()
  })
})
