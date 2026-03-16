import { beforeEach, describe, expect, it, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import Entry from './Entry.vue'

vi.mock('~/utils/dbQueries', () => ({
    addTag: vi.fn(),
    removeTag: vi.fn(),
}))

const stubs = {
    Icon: { template: '<span class="icon-stub" />' },
    TagEditor: {
        props: ['tags'],
        template: '<div class="tag-editor">{{ tags.join(",") }}</div>',
    },
    EntryPlugins: { template: '<div class="entry-plugins" />' },
    Sequence: { template: '<div class="sequence-stub" />' },
}

const baseProps = {
    id: 1,
    name: 'test.txt',
    path: '/home/user/test.txt',
    size: 1024,
    status: 'Complete',
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
    time_machine: null,
    platform_name: 'linux',
    platform_image_link: null,
    scenario_name: null,
    scenario_creation_time: null,
    scenario_description: null,
    sequence_duration: 120,
    sequence_distance: null,
    sequence_lat_starting_point_deg: null,
    sequence_lon_starting_point_deg: null,
    weather_cloudiness: null,
    weather_precipitation: null,
    weather_precipitation_deposits: null,
    weather_wind_intensity: null,
    weather_road_humidity: null,
    weather_fog: null,
    weather_snow: null,
    tags: ['important', 'work'],
}

describe('Entry Component', () => {
    beforeEach(() => {
        vi.clearAllMocks()
    })

    it('renders entry properties correctly', () => {
        const wrapper = mount(Entry, {
            props: baseProps,
            global: { stubs }
        })

        expect(wrapper.text()).toContain('test.txt')
        expect(wrapper.text()).toContain('/home/user/test.txt')
        expect(wrapper.text()).toContain('0.00 MB')
        expect(wrapper.text()).toContain('linux')
        expect(wrapper.text()).toContain('important')
        expect(wrapper.text()).toContain('work')
    })

    it('emits select event with entryID when clicked', async () => {
        const wrapper = mount(Entry, {
            props: { ...baseProps, id: 42, tags: [] },
            global: { stubs }
        })

        await wrapper.find('tr').trigger('click')

        expect(wrapper.emitted()).toHaveProperty('select')
        expect(wrapper.emitted('select')).toHaveLength(1)
        expect(wrapper.emitted('select')?.[0]).toEqual([42])
    })

    it('renders multiple tags with badge class', () => {
        const wrapper = mount(Entry, {
            props: { ...baseProps, name: 'document.pdf', path: '/docs/document.pdf', platform_name: 'windows', tags: ['tag1', 'tag2', 'tag3'] },
            global: { stubs }
        })

        expect(wrapper.find('.tag-editor').text()).toContain('tag1,tag2,tag3')
    })

    it('handles empty tags array', () => {
        const wrapper = mount(Entry, {
            props: { ...baseProps, name: 'file.txt', path: '/path/file.txt', platform_name: 'macos', tags: [] },
            global: { stubs }
        })

        expect(wrapper.find('.tag-editor').text()).toBe('')
    })

    it('has hover and cursor pointer classes on row', () => {
        const wrapper = mount(Entry, {
            props: { ...baseProps, path: '/test.txt', size: 100, tags: [] },
            global: { stubs }
        })

        const row = wrapper.find('tr')
        expect(row.classes()).toContain('cursor-pointer')
        expect(row.classes()).toContain('hover:bg-base-300')
    })

    it('shows the expanded sequence row when toggled', async () => {
        const wrapper = mount(Entry, {
            props: baseProps,
            global: { stubs }
        })

        await wrapper.find('button').trigger('click')

        expect(wrapper.find('.sequence-stub').exists()).toBe(true)
    })
})
