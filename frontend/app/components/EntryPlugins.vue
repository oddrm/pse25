<script setup lang="ts">
import { ref } from 'vue'
import { useNuxtApp } from '#app'
import type { Pinia } from 'pinia'


import { usePluginsStore } from '../../stores/plugins'
import type { PluginItem } from '../../stores/plugins'
import { useLogsStore } from '../../stores/logsStore'


const { $pinia } = useNuxtApp()

// ✅ BEIDE Stores mit Pinia initialisieren
const pluginsStore = usePluginsStore(($pinia as Pinia))
const logsStore = useLogsStore(($pinia as Pinia))

const props = defineProps<{
  entry: {
    entryID: number
    name: string
  }
}>()

const open = ref(false)

const runPluginOnEntry = async (plugin: PluginItem) => {
  if (plugin.isRunning) return

  plugin.isRunning = true
  plugin.progress = 0
  open.value = false

  const interval = setInterval(() => {
    if (plugin.progress! < 100) plugin.progress! += 5
  }, 150)

  await new Promise(resolve => setTimeout(resolve, 3000))

  clearInterval(interval)

  plugin.isRunning = false
  plugin.progress = 0

  logsStore.addLog(
    'info',
    `Plugin "${plugin.name}" wurde auf Datei "${props.entry.name}" angewendet.`
  )
}
</script>

<template>
  <div class="relative">
    <button
      class="bg-blue-500 text-white px-3 py-1 rounded hover:bg-blue-600"
      @click="open = !open"
    >
      Plugins
    </button>

    <div
      v-if="open"
      class="absolute right-0 mt-2 bg-white border rounded shadow z-20 w-56"
    >
      <button
        v-for="plugin in pluginsStore.plugins"
        :key="plugin.id"
        class="w-full text-left px-3 py-2 hover:bg-gray-100 disabled:opacity-50"
        :disabled="plugin.isRunning"
        @click="runPluginOnEntry(plugin)"
      >
        {{ plugin.name }}
      </button>
    </div>
  </div>
</template>
