<template>
  <tr class="cursor-pointer hover:bg-base-300 transition-colors duration-150" @click="$emit('select', props.id)">
    <td>
      <span :class="'flex items-center justify-center min-w-min badge font-medium ' + statusColor(props.status)">

      </span>
    </td>
    <td class="font-medium whitespace-nowrap">{{ props.name }}</td>
    <td class="text-xs text-gray-500 truncate max-w-40" :title="props.path">{{ props.path }}</td>
    <td>{{ (props.size / 1000 / 1000).toFixed(2) }} MB</td>
    <td>{{ props.platform_name }}</td>

    <td @click.stop class="min-w-20">
      <TagEditor :tags="localTags" @update="onTagsChange" />
    </td>

    <td @click.stop class="">
      <EntryPlugins :entry="{
        entryID: props.id,
        name: props.name
      }" />
    </td>

    <td @click.stop class="text-right pr-4">
      <button @click="toggle" class="btn btn-ghost btn-xs btn-circle hover:bg-base-200">
        <Icon v-if="!open" icon="garden:chevron-down-stroke-12" width="20" height="20" class="text-gray-500" />
        <Icon v-else icon="garden:chevron-up-stroke-12" width="20" height="20" class="text-primary" />
      </button>
    </td>
  </tr>

  <tr v-if="open" class="bg-base-50/50">
    <td :colspan="7" class="p-0 border-b border-base-300 shadow-inner">
      <div class="p-2 pl-4">
        <Sequence :entryID="props.id" :totalDuration="props.sequence_duration || 0" />
      </div>
    </td>
  </tr>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue' // 'watch' hinzugefügt
import { Icon } from '@iconify/vue'
import type { Entry, entryID } from '~/utils/entry'
import EntryPlugins from '~/components/EntryPlugins.vue'
import Sequence from '~/components/sequence.vue'
import TagEditor from '~/components/TagEditor.vue'
import { addTag, removeTag } from '~/utils/dbQueries'

const props = defineProps<Entry>()

defineEmits<{
  (e: 'select', id: entryID): void
}>()

// LOKALER STATE FÜR TAGS (Ersatz für Store)
// Wir erstellen eine Kopie, damit wir sie bearbeiten können.
const localTags = ref<string[]>([...props.tags])

const statusColor = (status: string) => {
  switch (status) {
    case 'Complete':
      return 'badge-success'
    case 'No MCAP Info':
      return 'badge-error'
    case 'Partial MCAP Info':
      return 'badge-warning'
    default:
      return 'badge-neutral'
  }
};

// Falls sich die Daten von der "Tabelle" ändern (z.B. durch Suche/Sortierung),
// müssen wir unsere lokale Kopie aktualisieren.
watch(() => props.tags, (newTags) => {
  localTags.value = [...newTags]
})

// Lokaler State für das Auf-/Zuklappen
const open = ref(false)

function toggle() {
  open.value = !open.value
}

/**
 * 5. Tag Update Logik (Lokal)
 * Da kein Store mehr da ist, ändern wir einfach unsere lokale Variable.
 * Das X funktioniert sofort, aber beim Neuladen der Seite sind die alten Tags wieder da.
 */
async function onTagsChange(newTags: string[]) {
  const old = props.tags || []
  // diff
  const toAdd = newTags.filter(t => !old.includes(t))
  const toRemove = old.filter(t => !newTags.includes(t))

  // optimistic update
  localTags.value = newTags

  try {
    for (const t of toAdd) {
      await addTag(props.id, t)
    }
    for (const t of toRemove) {
      await removeTag(props.id, t)
    }
    console.log(`[Entry ${props.id}] Tags synchronized with backend`)
  } catch (err) {
    console.error('Error syncing tags:', err)
  }
}
</script>