<template>
  <div v-if="entry" class="p-4 space-y-2">
    <h2 class="text-xl font-bold">Entry info</h2>

    <p><strong>Name:</strong> {{ entry.name }}</p>
    <p><strong>Path:</strong> {{ entry.path }}</p>
    <p><strong>Size:</strong> {{ entry.size }}</p>
    <p><strong>Platform:</strong> {{ entry.platform }}</p>
  </div>

  <div v-else class="p-4 text-gray-400">
    No entry selected
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { Sorting } from '~/utils/entryColumns'
import type { Entry, entryID } from '~/utils/entry'
import { fetchEntries } from '~/utils/dbQueries'

const props = defineProps<{
  entryID: entryID | null
}>()

const entry = ref<Entry | null>(null)

watch(
  () => props.entryID,
  (id) => {
    if (!id) {
      entry.value = null
      return
    }

    const entries = fetchEntries('', Sorting.Name, true, 1, 50)
    entry.value = entries.find(e => e.entryID === id) || null
  },
  { immediate: true }
)
</script>
