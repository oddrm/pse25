import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import TagEditor from './TagEditor.vue'

const iconStub = {
    template: '<span class="icon-stub" />',
}

describe('TagEditor', () => {
    it('shows a fallback when there are no tags', () => {
        const wrapper = mount(TagEditor, {
            props: { tags: [] },
            global: { stubs: { NuxtIcon: iconStub, Icon: iconStub } },
        })

        expect(wrapper.text()).toContain('No tags')
    })

    it('removes a tag when the remove button is pressed', async () => {
        const wrapper = mount(TagEditor, {
            props: { tags: ['a', 'b'] },
            global: { stubs: { NuxtIcon: iconStub, Icon: iconStub } },
        })

        await wrapper.findAll('button')[0]?.trigger('click')

        expect(wrapper.emitted('update')?.[0]).toEqual([['b']])
    })

    it('opens edit mode and seeds the input with the current tags', async () => {
        const wrapper = mount(TagEditor, {
            props: { tags: ['alpha', 'beta'] },
            global: { stubs: { NuxtIcon: iconStub, Icon: iconStub } },
        })

        await wrapper.findAll('button')[2]?.trigger('click')

        const input = wrapper.find('input')
        expect(input.exists()).toBe(true)
        expect((input.element as HTMLInputElement).value).toBe('alpha, beta')
    })

    it('saves cleaned unique tags', async () => {
        const wrapper = mount(TagEditor, {
            props: { tags: ['alpha'] },
            global: { stubs: { NuxtIcon: iconStub, Icon: iconStub } },
        })

        await wrapper.find('button[data-tip="Edit tags"]').trigger('click')
        await wrapper.find('input').setValue(' alpha, beta, alpha, , gamma ')
        await wrapper.find('.btn-success').trigger('click')

        expect(wrapper.emitted('update')?.[0]).toEqual([['alpha', 'beta', 'gamma']])
        expect(wrapper.find('input').exists()).toBe(false)
    })

    it('cancels editing without emitting', async () => {
        const wrapper = mount(TagEditor, {
            props: { tags: ['alpha'] },
            global: { stubs: { NuxtIcon: iconStub, Icon: iconStub } },
        })

        await wrapper.find('button[data-tip="Edit tags"]').trigger('click')
        await wrapper.find('.text-error').trigger('click')

        expect(wrapper.emitted('update')).toBeUndefined()
        expect(wrapper.find('input').exists()).toBe(false)
    })
})
