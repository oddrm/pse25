<template>
  <tr
    class="cursor-pointer hover:bg-base-300"
    @click="$emit('select', props.entryID)"
  >
    <td>{{ props.name }}</td>
    <td>{{ props.path }}</td>
    <td>{{ props.size }}</td>
    <td>{{ props.platform }}</td>
    <td>
      <span
        v-for="(tag, index) in props.tags"
        :key="index"
        class="badge mr-1 bg-blue-800 text-white"
      >
        {{ tag }}
      </span>
    </td>

    <!-- NEU: Plugin Button -->
    <td @click.stop class="scale-85">
      <EntryPlugins
        :entry="{
          entryID: props.entryID,
          name: props.name
        }"
      />
    </td>

    <!--Pfeile-->
      <td @click.stop>
        <button @click="toggle" class="flex items-center">
          <Icon
            v-if="!open"
            icon="garden:chevron-down-stroke-12"
            width="16"
            height="16"
          />
          <Icon
            v-else
            icon="garden:chevron-up-stroke-12"
            width="16"
            height="16"
          />
        </button>
      </td>
    </tr>

    <!--Opened-->
    <tr v-if="open">
      <td :colspan="7">
        <Sequence />
      </td>
    </tr>
</template>

<script setup lang="ts">
import type { Entry, entryID } from '~/utils/entry'
import EntryPlugins from '~/components/EntryPlugins.vue'
import Sequence from '~/components/sequence.vue'
import { Icon } from '@iconify/vue'

const props = defineProps<Entry>()

defineEmits<{
  (e: 'select', id: entryID): void
}>()

const open = ref(false)

function toggle() {
  open.value = !open.value
}
</script>
