import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import Entry from './Entry.vue'

describe('Entry Component', () => {
    it('renders entry properties correctly', () => {
        const wrapper = mount(Entry, {
            props: {
                name: 'test.txt',
                path: '/home/user/test.txt',
                size: 1024,
                platform: 'linux',
                tags: ['important', 'work'],
                entryID: 1
            }
        })

        expect(wrapper.text()).toContain('test.txt')
        expect(wrapper.text()).toContain('/home/user/test.txt')
        expect(wrapper.text()).toContain('1024')
        expect(wrapper.text()).toContain('linux')
        expect(wrapper.text()).toContain('important')
        expect(wrapper.text()).toContain('work')
    })

    it('emits select event with entryID when clicked', async () => {
        const wrapper = mount(Entry, {
            props: {
                name: 'test.txt',
                path: '/home/user/test.txt',
                size: 1024,
                platform: 'linux',
                tags: [],
                entryID: 42
            }
        })

        await wrapper.find('tr').trigger('click')

        expect(wrapper.emitted()).toHaveProperty('select')
        expect(wrapper.emitted('select')).toHaveLength(1)
        expect(wrapper.emitted('select')?.[0]).toEqual([42])
    })

    it('renders multiple tags with badge class', () => {
        const wrapper = mount(Entry, {
            props: {
                name: 'document.pdf',
                path: '/docs/document.pdf',
                size: 2048,
                platform: 'windows',
                tags: ['tag1', 'tag2', 'tag3'],
                entryID: 1
            }
        })

        const badges = wrapper.findAll('.badge')
        expect(badges).toHaveLength(3)
        expect(badges[0]!.text()).toBe('tag1')
        expect(badges[1]!.text()).toBe('tag2')
        expect(badges[2]!.text()).toBe('tag3')
    })

    it('handles empty tags array', () => {
        const wrapper = mount(Entry, {
            props: {
                name: 'file.txt',
                path: '/path/file.txt',
                size: 512,
                platform: 'macos',
                tags: [],
                entryID: 1
            }
        })

        const badges = wrapper.findAll('.badge')
        expect(badges).toHaveLength(0)
    })

    it('has hover and cursor pointer classes on row', () => {
        const wrapper = mount(Entry, {
            props: {
                name: 'test.txt',
                path: '/test.txt',
                size: 100,
                platform: 'linux',
                tags: [],
                entryID: 1
            }
        })

        const row = wrapper.find('tr')
        expect(row.classes()).toContain('cursor-pointer')
        expect(row.classes()).toContain('hover:bg-base-300')
    })
})
