<script setup lang="ts">
import { onMounted } from 'vue'
import usePluginsStore from '../../stores/plugins'
import type { PluginItem } from '../../stores/plugins'
import { useLogsStore } from '../../stores/logsStore'

const pluginsStore = usePluginsStore()
const logsStore = useLogsStore()

onMounted(() => {
  pluginsStore.loadTestPlugins()
})

// Funktion für Plugin-Task
const runPlugin = async (plugin: PluginItem) => {
  if (plugin.isRunning) return // Schutz, damit es nicht erneut startet

  plugin.isRunning = true
  plugin.progress = 0

  // Fortschritt simulieren
  const interval = setInterval(() => {
    if (plugin.progress! < 100) plugin.progress! += 2
  }, 100)

  // Mock Task 5 Sekunden
  await new Promise((resolve) => setTimeout(resolve, 5000))

  clearInterval(interval)
  plugin.progress = 100

  setTimeout(() => {
    plugin.isRunning = false
    plugin.progress = 0

    // Log hinzufügen
    logsStore.addLog(
      'info',
      `Plugin "${plugin.name}" wurde erfolgreich auf alle Dateien angewendet.`
    )
  }, 500)
}
</script>

<template>
  <div class="p-6">
    <h1 class="text-xl font-bold mb-4">Plugins</h1>

    <!-- Fortschrittsbalken -->
    <div v-if="pluginsStore.runningPlugin" class="mb-4">
      <div class="text-sm mb-1">
        {{ pluginsStore.runningPlugin.pluginName }} läuft für {{ pluginsStore.runningPlugin.entryName }}
      </div>
      <div class="w-full bg-gray-200 h-3 rounded overflow-hidden">
        <div
          class="bg-blue-500 h-3 transition-all duration-100"
          :style="{ width: pluginsStore.runningPlugin.progress + '%' }"
        ></div>
      </div>
    </div>

    <!-- Tabelle -->
    <table class="w-full border-collapse border border-gray-300">
      <thead>
        <tr class="bg-gray-100">
          <th class="border border-gray-300 p-2 text-left"># / Name</th>
          <th class="border border-gray-300 p-2 text-left">Beschreibung</th>
          <th class="border border-gray-300 p-2 text-left">Fortschritt</th>
          <th class="border border-gray-300 p-2 text-left">Aktion</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="plugin in pluginsStore.plugins" :key="plugin.id">
          <td class="border border-gray-300 p-2">{{ plugin.id }} - {{ plugin.name }}</td>
          <td class="border border-gray-300 p-2">{{ plugin.description }}</td>

          <!-- Fortschrittsanzeige pro Plugin -->
          <td class="border border-gray-300 p-2 w-64">
            <div v-if="plugin.isRunning" class="w-full bg-gray-200 rounded h-4">
              <div
                class="bg-blue-500 h-4 rounded transition-all duration-100"
                :style="{ width: plugin.progress + '%' }"
              ></div>
            </div>
            <div v-else class="text-gray-500">-</div>
          </td>

          <!-- Button pro Plugin -->
          <td class="border border-gray-300 p-2">
            <button
              class="bg-blue-500 text-white px-3 py-1 rounded hover:bg-blue-600 disabled:opacity-50"
              :disabled="plugin.isRunning"
              @click="runPlugin(plugin)"
            >
              Auf alle anwenden
            </button>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>
