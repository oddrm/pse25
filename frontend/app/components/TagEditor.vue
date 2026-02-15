<template>
  <div class="flex flex-col items-start gap-2">
    
    <div v-if="!isEditing" class="flex flex-wrap gap-1 items-center min-h-[2rem]">
      <span 
        v-for="(tag, index) in tags" 
        :key="index" 
        class="badge badge-primary text-white gap-1 pr-1"
      >
        {{ tag }}
        <button @click.stop="removeTag(index)" class="btn btn-ghost btn-xs btn-circle text-white w-4 h-4 min-h-0 opacity-70 hover:opacity-100">
          <Icon name="solar:close-circle-bold" size="12" />
        </button>
      </span>

      <button 
        @click="startEditing" 
        class="btn btn-xs btn-ghost btn-circle text-gray-500 hover:text-primary tooltip tooltip-right"
        data-tip="Tags bearbeiten"
      >
        <Icon name="solar:pen-new-square-linear" size="16" />
      </button>

      <span v-if="tags.length === 0" class="text-xs text-gray-400 italic">Keine Tags</span>
    </div>

    <div v-else class="join w-full max-w-md">
      <input
        ref="inputRef"
        v-model="editString"
        class="input input-sm input-bordered join-item w-full"
        placeholder="Tags mit Komma trennen (z.B. Wald, Fehler)"
        @keydown.enter="saveTags"
        @keydown.esc="cancelEdit"
      />
      <button @click="saveTags" class="btn btn-sm btn-success text-white join-item">
        <Icon name="solar:check-circle-bold" size="18" />
      </button>
      <button @click="cancelEdit" class="btn btn-sm btn-ghost text-error join-item">
        <Icon name="solar:close-circle-linear" size="18" />
      </button>
    </div>

  </div>
</template>

<script setup lang="ts">
import { ref, nextTick } from 'vue'

const props = defineProps<{
  tags: string[]
}>()

const emit = defineEmits<{
  (e: 'update', newTags: string[]): void
}>()

const isEditing = ref(false)
const editString = ref("")
const inputRef = ref<HTMLInputElement | null>(null)

// --- MODUS WECHSELN ---

const startEditing = async () => {
  // Array zu String umwandeln: ["a", "b"] -> "a, b"
  editString.value = props.tags.join(", ")
  isEditing.value = true
  
  // Fokus automatisch ins Feld setzen
  await nextTick()
  inputRef.value?.focus()
}

const cancelEdit = () => {
  isEditing.value = false
}


const removeTag = (index: number) => {
  // Kopie erstellen, Element löschen, emitten
  const newTags = [...props.tags]
  newTags.splice(index, 1)
  emit('update', newTags)
}

const saveTags = () => {
  // String am Komma teilen und bereinigen
  const rawTags = editString.value.split(",")
  
  const cleanedTags = rawTags
    .map(t => t.trim())       // Leerzeichen entfernen
    .filter(t => t.length > 0) // Leere Einträge entfernen
    // Duplikate entfernen
    .filter((val, idx, self) => self.indexOf(val) === idx)

  emit('update', cleanedTags)
  isEditing.value = false
}
</script>