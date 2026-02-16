<template>
  <tr
    class="cursor-pointer hover:bg-base-300 transition-colors duration-150"
    @click="$emit('select', props.entryID)"
  >
    <td class="font-medium">{{ props.name }}</td>
    <td class="text-xs text-gray-500 truncate max-w-[150px]" :title="props.path">{{ props.path }}</td>
    <td>{{ props.size }}</td> 
    <td>{{ props.platform }}</td>

    <td @click.stop class="min-w-[200px]">
      <TagEditor 
        :tags="localTags" 
        @update="onTagsChange" 
      />
    </td>

    <td @click.stop class="scale-90">
      <EntryPlugins
        :entry="{
          entryID: props.entryID,
          name: props.name
        }"
      />
    </td>

    <td @click.stop class="text-right pr-4">
      <button 
        @click="toggle" 
        class="btn btn-ghost btn-xs btn-circle hover:bg-base-200"
      >
        <Icon
          v-if="!open"
          icon="garden:chevron-down-stroke-12"
          width="20"
          height="20"
          class="text-gray-500"
        />
        <Icon
          v-else
          icon="garden:chevron-up-stroke-12"
          width="20"
          height="20"
          class="text-primary"
        />
      </button>
    </td>
  </tr>

  <tr v-if="open" class="bg-base-50/50">
    <td :colspan="7" class="p-0 border-b border-base-300 shadow-inner">
      <div class="p-2 pl-4">
        <Sequence 
          :entryID="props.entryID" 
          :totalDuration="props.duration || 0" 
        />
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

const props = defineProps<Entry>()

defineEmits<{
  (e: 'select', id: entryID): void
}>()

// LOKALER STATE FÜR TAGS (Ersatz für Store)
// Wir erstellen eine Kopie, damit wir sie bearbeiten können.
const localTags = ref<string[]>([...props.tags])

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
function onTagsChange(newTags: string[]) {
  // Update der lokalen Anzeige -> X funktioniert sofort
  localTags.value = newTags
  
  console.log(`[Entry ${props.entryID}] Local tags updated to:`, newTags)
}
</script>