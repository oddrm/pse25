import { describe, it, expect } from 'vitest'
import type { Entry, entryID } from './entry'

describe('Entry Type', () => {
    it('should create a valid Entry object', () => {
        const entry: Entry = {
            name: 'test.txt',
            path: '/home/user/test.txt',
            size: 1024,
            platform: 'linux',
            tags: ['important', 'work'],
            entryID: 1 as entryID
        }

        expect(entry.name).toBe('test.txt')
        expect(entry.path).toBe('/home/user/test.txt')
        expect(entry.size).toBe(1024)
        expect(entry.platform).toBe('linux')
        expect(entry.tags).toHaveLength(2)
        expect(entry.tags).toContain('important')
        expect(entry.entryID).toBe(1)
    })

    it('should handle empty tags array', () => {
        const entry: Entry = {
            name: 'file.pdf',
            path: '/docs/file.pdf',
            size: 2048,
            platform: 'windows',
            tags: [],
            entryID: 2 as entryID
        }

        expect(entry.tags).toEqual([])
        expect(entry.tags).toHaveLength(0)
    })

    it('should handle entryID as number type', () => {
        const id: entryID = 42 as entryID

        expect(typeof id).toBe('number')
        expect(id).toBe(42)
    })
})
