import { beforeEach, describe, expect, it, vi } from 'vitest'
import { Sorting } from './entryColumns'
import {
    addSensor,
    addSequence,
    addTag,
    fetchAllSensors,
    fetchEntries,
    fetchEntry,
    fetchSensors,
    fetchSequences,
    fetchTopics,
    removeSensor,
    removeSequence,
    removeTag,
    updateMetadata,
    updateSensor,
    updateSequence,
} from './dbQueries'

const fetchMock = vi.fn()

vi.stubGlobal('$fetch', fetchMock)

describe('dbQueries', () => {
    beforeEach(() => {
        fetchMock.mockReset()
    })

    it('fetchEntries forwards the expected query params', async () => {
        const response = [[{ id: 1 }], 5]
        fetchMock.mockResolvedValue(response)

        const result = await fetchEntries('term', Sorting.Name, true, 2, 25)

        expect(result).toBe(response)
        expect(fetchMock).toHaveBeenCalledWith('/backend/entries', {
            method: 'GET',
            query: {
                search_string: 'term',
                sort_by: 'Name',
                ascending: 'true',
                page: 2,
                page_size: 25,
            },
        })
    })

    it('fetchEntries normalizes thrown errors', async () => {
        fetchMock.mockRejectedValue(new Error('backend down'))

        await expect(fetchEntries('', Sorting.Path, false, 0, 10)).rejects.toThrow('backend down')
    })

    it('fetches entry-related resources with expected endpoints', async () => {
        const sequences = { 1: { id: 1, entry_id: 7 } }
        const sensors = { 2: { id: 2, entry_id: 7 } }
        const topics = { 3: { id: 3, entry_id: 7 } }
        const entry = { id: 7, name: 'entry' }

        fetchMock
            .mockResolvedValueOnce(sequences)
            .mockResolvedValueOnce(sensors)
            .mockResolvedValueOnce(topics)
            .mockResolvedValueOnce(entry)
            .mockResolvedValueOnce(sensors)

        await expect(fetchSequences(7)).resolves.toBe(sequences)
        await expect(fetchSensors(7)).resolves.toBe(sensors)
        await expect(fetchTopics(7)).resolves.toBe(topics)
        await expect(fetchEntry(7)).resolves.toBe(entry)
        await expect(fetchAllSensors()).resolves.toBe(sensors)

        expect(fetchMock).toHaveBeenNthCalledWith(1, '/backend/entries/7/sequences/tx/0')
        expect(fetchMock).toHaveBeenNthCalledWith(2, '/backend/entries/7/sensors/tx/0')
        expect(fetchMock).toHaveBeenNthCalledWith(3, '/backend/entries/7/topics/tx/0')
        expect(fetchMock).toHaveBeenNthCalledWith(4, '/backend/entries/7/tx/0')
        expect(fetchMock).toHaveBeenNthCalledWith(5, '/backend/sensors/tx/0')
    })

    it('wraps resource fetch errors with resource-specific messages', async () => {
        fetchMock
            .mockRejectedValueOnce({})
            .mockRejectedValueOnce({})
            .mockRejectedValueOnce({})
            .mockRejectedValueOnce({})

        await expect(fetchSensors(1)).rejects.toThrow('error fetching sensors')
        await expect(fetchAllSensors()).rejects.toThrow('error fetching all sensors')
        await expect(fetchTopics(1)).rejects.toThrow('error fetching topics')
        await expect(fetchEntry(1)).rejects.toThrow('error fetching entry')
    })

    it('sends tag mutations with the expected methods and body', async () => {
        fetchMock.mockResolvedValue(undefined)

        await addTag(3, 'alpha')
        await removeTag(3, 'alpha')

        expect(fetchMock).toHaveBeenNthCalledWith(1, '/backend/entries/3/tags/tx/0', {
            method: 'PUT',
            body: 'alpha',
        })
        expect(fetchMock).toHaveBeenNthCalledWith(2, '/backend/entries/3/tags/tx/0', {
            method: 'DELETE',
            body: 'alpha',
        })
    })

    it('sends sequence mutations with the expected payload', async () => {
        const payload = {
            description: 'segment',
            start_timestamp: 1,
            end_timestamp: 3,
            tags: ['x'],
        }
        fetchMock.mockResolvedValueOnce(77).mockResolvedValue(undefined)

        await expect(addSequence(9, payload)).resolves.toBe(77)
        await updateSequence(9, 77, payload)
        await removeSequence(9, 77)

        expect(fetchMock).toHaveBeenNthCalledWith(1, '/backend/entries/9/sequences/tx/0', {
            method: 'POST',
            body: payload,
        })
        expect(fetchMock).toHaveBeenNthCalledWith(2, '/backend/entries/9/sequences/77/tx/0', {
            method: 'PUT',
            body: payload,
        })
        expect(fetchMock).toHaveBeenNthCalledWith(3, '/backend/entries/9/sequences/77/tx/0', {
            method: 'DELETE',
        })
    })

    it('sends metadata and sensor mutations with the expected payload', async () => {
        const metadata = { platform_name: 'car' }
        const sensor = {
            sensor_name: 'cam',
            manufacturer: 'acme',
            sensor_type: 'camera',
            ros_topics: ['/image'],
            custom_parameters: { fps: 30 },
        }
        fetchMock.mockResolvedValueOnce(undefined).mockResolvedValueOnce(11).mockResolvedValue(undefined)

        await updateMetadata(5, metadata)
        await expect(addSensor(5, sensor)).resolves.toBe(11)
        await updateSensor(5, 11, sensor)
        await removeSensor(11)

        expect(fetchMock).toHaveBeenNthCalledWith(1, '/backend/entries/5/metadata/tx/0', {
            method: 'PUT',
            body: metadata,
        })
        expect(fetchMock).toHaveBeenNthCalledWith(2, '/backend/entries/5/sensors/tx/0', {
            method: 'POST',
            body: sensor,
        })
        expect(fetchMock).toHaveBeenNthCalledWith(3, '/backend/entries/5/sensors/11/tx/0', {
            method: 'PUT',
            body: sensor,
        })
        expect(fetchMock).toHaveBeenNthCalledWith(4, '/backend/sensors/11/tx/0', {
            method: 'DELETE',
        })
    })

    it('wraps mutation errors with fallback messages', async () => {
        fetchMock
            .mockRejectedValueOnce({})
            .mockRejectedValueOnce({})
            .mockRejectedValueOnce({})
            .mockRejectedValueOnce({})
            .mockRejectedValueOnce({})
            .mockRejectedValueOnce({})
            .mockRejectedValueOnce({})
            .mockRejectedValueOnce({})

        await expect(addTag(1, 'x')).rejects.toThrow('error adding tag')
        await expect(removeTag(1, 'x')).rejects.toThrow('error removing tag')
        await expect(addSequence(1, { description: '', start_timestamp: 0, end_timestamp: 1, tags: [] })).rejects.toThrow('error adding sequence')
        await expect(updateSequence(1, 2, { description: '', start_timestamp: 0, end_timestamp: 1, tags: [] })).rejects.toThrow('error updating sequence')
        await expect(removeSequence(1, 2)).rejects.toThrow('error removing sequence')
        await expect(updateMetadata(1, {})).rejects.toThrow('error updating metadata')
        await expect(addSensor(1, { sensor_name: 'a', manufacturer: null, sensor_type: null, ros_topics: [], custom_parameters: null })).rejects.toThrow('error adding sensor')
        await expect(updateSensor(1, 2, { sensor_name: 'a', manufacturer: null, sensor_type: null, ros_topics: [], custom_parameters: null })).rejects.toThrow('error updating sensor')
    })

    it('wraps sensor deletion errors', async () => {
        fetchMock.mockRejectedValue({})

        await expect(removeSensor(2)).rejects.toThrow('error removing sensor')
    })
})
