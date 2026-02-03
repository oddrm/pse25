<script setup lang="ts">
import { ref, nextTick, onMounted } from 'vue'
import { useNuxtApp } from '#app'
import type { Pinia } from 'pinia'
import { onClickOutside } from '@vueuse/core'

import { usePluginsStore } from '../../stores/plugins'
import type { PluginItem } from '../../stores/plugins'
import { useLogsStore } from '../../stores/logsStore'

// Pinia Stores initialisieren
const { $pinia } = useNuxtApp()
const pluginsStore = usePluginsStore($pinia as Pinia)
const logsStore = useLogsStore($pinia as Pinia)

// Props
const props = defineProps<{
  entry: {
    entryID: number
    name: string
  }
}>()

// Dropdown State
const open = ref(false)
const isMounted = ref(false)
const dropdownRef = ref<HTMLElement | null>(null)
const buttonRef = ref<HTMLElement | null>(null)
const dropdownStyle = ref({
  top: '0px',
  left: '0px',
  visibility: 'hidden' as 'hidden' | 'visible'
})

// Plugins automatisch laden, falls noch nicht geschehen
onMounted(() => {
  isMounted.value = true
  if (pluginsStore.plugins.length === 0) {
    pluginsStore.loadTestPlugins()
  }

  // Klick außerhalb → Dropdown schließen
  onClickOutside(dropdownRef, () => {
    open.value = false
  })
})

// Dropdown Position aktualisieren
const updatePosition = () => {
  if (buttonRef.value) {
    const rect = buttonRef.value.getBoundingClientRect()
    dropdownStyle.value = {
      top: `${rect.bottom + window.scrollY + 4}px`,
      left: `${rect.left + window.scrollX}px`,
      visibility: 'visible'
    }
  }
}

// Dropdown umschalten
const toggleDropdown = async () => {
  if (open.value) {
    open.value = false
    return
  }

  open.value = true
  
  // Position berechnen
  await nextTick()
  requestAnimationFrame(() => {
    updatePosition()
  })
}

// Plugin auf einzelnen Entry ausführen
const runPluginOnEntry = async (plugin: PluginItem) => {
  if (plugin.isRunning) return
  plugin.isRunning = true
  open.value = false

  pluginsStore.startPlugin(plugin, props.entry.name)
  
  // Simulierte Verarbeitung (ersetze durch echte Logik)
  await new Promise(resolve => setTimeout(resolve, 1000))
  plugin.isRunning = false

}
</script>

<template>
  <div class="relative inline-block">
    <button
      ref="buttonRef"
      type="button"
      class="bg-blue-500 text-white px-3 py-1 rounded hover:bg-blue-600 transition-colors text-sm font-medium"
      @click="toggleDropdown"
    >
      Plugins
    </button>
  </div>

  <Teleport to="body">
    <div
      v-if="isMounted && open"
      ref="dropdownRef"
      class="fixed bg-white border border-gray-300 rounded shadow-md z-[9999] w-64 flex flex-col overflow-hidden text-sm"
      :style="dropdownStyle"
    >
      <div v-if="pluginsStore.plugins && pluginsStore.plugins.length > 0" class="py-1">
        <button
          v-for="plugin in pluginsStore.plugins"
          :key="plugin.id"
          class="w-full text-left px-4 py-2 hover:bg-gray-50 disabled:opacity-50 border-b last:border-b-0 border-gray-200 flex flex-col"
          :disabled="plugin.isRunning"
          @click="runPluginOnEntry(plugin)"
        >
          <span class="font-normal text-gray-800">{{ plugin.name }}</span>
          <span v-if="plugin.isRunning" class="text-[10px] text-blue-500 uppercase font-bold mt-1">Verarbeite...</span>
        </button>
      </div>
      <div v-else class="p-3 text-xs text-gray-500 text-center">
        Keine Plugins verfügbar
      </div>
    </div>
  </Teleport>
</template>

