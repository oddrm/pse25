import { afterAll, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

const fetchSequencesMock = vi.fn()
const addSequenceMock = vi.fn()
const updateSequenceMock = vi.fn()
const removeSequenceMock = vi.fn()

vi.mock('~/utils/dbQueries', () => ({
  fetchEntries: vi.fn(),
  fetchSequences: (...args: unknown[]) => fetchSequencesMock(...args),
  addSequence: (...args: unknown[]) => addSequenceMock(...args),
  updateSequence: (...args: unknown[]) => updateSequenceMock(...args),
  removeSequence: (...args: unknown[]) => removeSequenceMock(...args),
}))

import { useSequencesStore } from './sequenceStore'

describe('sequenceStore', () => {
  const originalClient = (globalThis as any).process?.client

  beforeEach(() => {
    setActivePinia(createPinia())
    fetchSequencesMock.mockReset()
    addSequenceMock.mockReset()
    updateSequenceMock.mockReset()
    removeSequenceMock.mockReset()
    ;(globalThis as any).process.client = true
  })

  afterAll(() => {
    ;(globalThis as any).process.client = originalClient
  })

  it('init marks the store as initialized on client', async () => {
    const store = useSequencesStore()

    await store.init()
    await store.init()

    expect(store._inited).toBe(true)
  })

  it('loadForEntries loads unique entries and skips cached ones', async () => {
    const store = useSequencesStore()
    fetchSequencesMock
      .mockResolvedValueOnce({
        1: { id: 1, entry_id: 10, description: 'a', start_timestamp: 0, end_timestamp: 1, created_at: '', updated_at: '', tags: [] },
      })
      .mockResolvedValueOnce({
        2: { id: 2, entry_id: 20, description: 'b', start_timestamp: 1, end_timestamp: 2, created_at: '', updated_at: '', tags: ['x'] },
      })

    await store.loadForEntries([10, 10, 20])
    await store.loadForEntries([10, 20])

    expect(fetchSequencesMock).toHaveBeenCalledTimes(2)
    expect(store.loaded_entry_ids).toEqual([10, 20])
    expect(store.byEntry(10)).toHaveLength(1)
    expect(store.byEntry(20)).toHaveLength(1)
  })

  it('loadForEntries forces a refresh when requested', async () => {
    const store = useSequencesStore()
    store.loaded_entry_ids = [10]
    store.sequences = [{ id: 1, entry_id: 10, description: 'old', start_timestamp: 0, end_timestamp: 1, created_at: '', updated_at: '', tags: [] }]
    fetchSequencesMock.mockResolvedValue({
      2: { id: 2, entry_id: 10, description: 'new', start_timestamp: 3, end_timestamp: 4, created_at: '', updated_at: '', tags: [] },
    })

    await store.loadForEntries([10], true)

    expect(store.byEntry(10)).toEqual([
      { id: 2, entry_id: 10, description: 'new', start_timestamp: 3, end_timestamp: 4, created_at: '', updated_at: '', tags: [] },
    ])
  })

  it('loadForEntries logs and continues on fetch errors', async () => {
    const store = useSequencesStore()
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    fetchSequencesMock.mockRejectedValue(new Error('boom'))

    await store.loadForEntries([10])

    expect(errorSpy).toHaveBeenCalled()
    expect(store.sequences).toEqual([])
  })

  it('add creates a new sequence and refreshes the entry', async () => {
    const store = useSequencesStore()
    const refreshSpy = vi.spyOn(store, 'loadForEntries').mockResolvedValue()
    addSequenceMock.mockResolvedValue(44)

    await store.add({
      entry_id: 5,
      name: 'Loop',
      description: 'segment',
      start_timestamp: 10,
      end_timestamp: 20,
      tags: ['x'],
    })

    expect(addSequenceMock).toHaveBeenCalledWith(5, {
      description: 'segment',
      start_timestamp: 10,
      end_timestamp: 20,
      tags: ['x'],
    })
    expect(store.sequences[0]).toMatchObject({
      id: 44,
      entry_id: 5,
      name: 'Loop',
      description: 'segment',
    })
    expect(refreshSpy).toHaveBeenCalledWith([5], true)
  })

  it('add prevents duplicate IDs and warns', async () => {
    const store = useSequencesStore()
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})
    addSequenceMock.mockResolvedValue(44)
    store.sequences = [{ id: 44, entry_id: 5, name: 'Loop', description: 'segment', start_timestamp: 1, end_timestamp: 2, created_at: '', updated_at: '', tags: [] }]

    await store.add({
      entry_id: 5,
      name: 'Loop',
      description: 'segment',
      start_timestamp: 10,
      end_timestamp: 20,
      tags: [],
    })

    expect(warnSpy).toHaveBeenCalled()
    expect(store.sequences).toHaveLength(1)
  })

  it('add logs backend failures', async () => {
    const store = useSequencesStore()
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    addSequenceMock.mockRejectedValue(new Error('bad'))

    await store.add({
      entry_id: 5,
      name: 'Loop',
      description: 'segment',
      start_timestamp: 10,
      end_timestamp: 20,
      tags: [],
    })

    expect(errorSpy).toHaveBeenCalled()
  })

  it('update persists changes and refreshes the entry', async () => {
    const store = useSequencesStore()
    const refreshSpy = vi.spyOn(store, 'loadForEntries').mockResolvedValue()
    updateSequenceMock.mockResolvedValue(undefined)
    store.sequences = [{ id: 4, entry_id: 5, name: 'Loop', description: 'segment', start_timestamp: 10, end_timestamp: 20, created_at: '', updated_at: '', tags: ['x'] }]

    await store.update({
      id: 4,
      entry_id: 5,
      name: 'New',
      description: 'updated',
      start_timestamp: 11,
      end_timestamp: 21,
      created_at: '',
      updated_at: '',
      tags: [],
    })

    expect(updateSequenceMock).toHaveBeenCalledWith(5, 4, {
      description: 'updated',
      start_timestamp: 11,
      end_timestamp: 21,
      tags: [],
    })
    expect(store.sequences[0]?.name).toBe('New')
    expect(refreshSpy).toHaveBeenCalledWith([5], true)
  })

  it('update exits when the sequence is not present', async () => {
    const store = useSequencesStore()

    await store.update({
      id: 4,
      entry_id: 5,
      name: 'New',
      description: 'updated',
      start_timestamp: 11,
      end_timestamp: 21,
      created_at: '',
      updated_at: '',
      tags: [],
    })

    expect(updateSequenceMock).not.toHaveBeenCalled()
  })

  it('update logs backend failures', async () => {
    const store = useSequencesStore()
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    updateSequenceMock.mockRejectedValue(new Error('bad'))
    store.sequences = [{ id: 4, entry_id: 5, name: 'Loop', description: 'segment', start_timestamp: 10, end_timestamp: 20, created_at: '', updated_at: '', tags: ['x'] }]

    await store.update({
      id: 4,
      entry_id: 5,
      name: 'New',
      description: 'updated',
      start_timestamp: 11,
      end_timestamp: 21,
      created_at: '',
      updated_at: '',
      tags: [],
    })

    expect(errorSpy).toHaveBeenCalled()
  })

  it('remove refreshes the parent entry and ignores invalid IDs', async () => {
    const store = useSequencesStore()
    const refreshSpy = vi.spyOn(store, 'loadForEntries').mockResolvedValue()
    removeSequenceMock.mockResolvedValue(undefined)
    store.sequences = [{ id: 4, entry_id: 5, name: 'Loop', description: 'segment', start_timestamp: 10, end_timestamp: 20, created_at: '', updated_at: '', tags: [] }]

    await store.remove(undefined as unknown as number)
    await store.remove(99)
    await store.remove(4)

    expect(removeSequenceMock).toHaveBeenCalledWith(5, 4)
    expect(refreshSpy).toHaveBeenCalledWith([5], true)
  })

  it('remove logs backend failures', async () => {
    const store = useSequencesStore()
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    removeSequenceMock.mockRejectedValue(new Error('bad'))
    store.sequences = [{ id: 4, entry_id: 5, name: 'Loop', description: 'segment', start_timestamp: 10, end_timestamp: 20, created_at: '', updated_at: '', tags: [] }]

    await store.remove(4)

    expect(errorSpy).toHaveBeenCalled()
  })

  it('clearAll resets the in-memory list', () => {
    const store = useSequencesStore()
    store.sequences = [{ id: 4, entry_id: 5, name: 'Loop', description: 'segment', start_timestamp: 10, end_timestamp: 20, created_at: '', updated_at: '', tags: [] }]

    store.clearAll()

    expect(store.sequences).toEqual([])
  })
})
