<script setup lang="ts">
import { onMounted } from 'vue'
import { usePluginsStore } from '../../stores/plugins'

const pluginsStore = usePluginsStore()

onMounted(() => {
  pluginsStore.loadTestPlugins()
})

// Nur den Store aufrufen, keine lokale Simulation!
const runGlobal = (id: number) => {
  pluginsStore.startPlugin(id) 
}
</script>

<template>
  <div class="p-6">
    <h1 class="text-xl font-bold mb-4">Plugin Management</h1>

    <div v-if="pluginsStore.runningPlugins.length > 0" class="mb-8 space-y-3">
  <h2 class="text-sm font-semibold text-gray-600 uppercase tracking-wider">
    Aktive Einzelprozesse
  </h2>

  <div 
    v-for="running in pluginsStore.runningPlugins" 
    :key="running.runId" 
    class="p-4 bg-white border rounded-lg shadow-sm"
  >
    <div class="flex justify-between text-sm mb-2">
      <span class="font-medium">
        {{ running.pluginName }} <span class="text-gray-400">für</span> {{ running.entryName }}
      </span>
      <span class="text-blue-600 font-bold">{{ running.progress }}%</span>
    </div>
    
    <div class="w-full bg-gray-100 h-2 rounded-full overflow-hidden">
      <div
        class="bg-blue-500 h-full transition-all duration-150"
        :style="{ width: running.progress + '%' }"
      ></div>
    </div>
  </div>
</div>

    <div class="bg-white rounded-lg shadow overflow-hidden border border-gray-200">
      <table class="w-full border-collapse">
        <thead>
          <tr class="bg-gray-50 border-b border-gray-200">
            <th class="p-4 text-left text-xs font-semibold text-gray-600 uppercase">Plugin</th>
            <th class="p-4 text-left text-xs font-semibold text-gray-600 uppercase">Beschreibung</th>
            <th class="p-4 text-left text-xs font-semibold text-gray-600 uppercase">Globaler Status</th>
            <th class="p-4 text-right text-xs font-semibold text-gray-600 uppercase">Aktion</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-gray-200">
          <tr v-for="plugin in pluginsStore.plugins" :key="plugin.id">
            <td class="p-4 font-medium text-gray-900">{{ plugin.name }}</td>
            <td class="p-4 text-sm text-gray-500">{{ plugin.description }}</td>
            
            <td class="p-4 w-64">
              <div v-if="plugin.isGlobalRunning">
                <div class="flex items-center gap-3">
                  <div class="flex-1 bg-gray-100 h-2 rounded-full overflow-hidden">
                    <div 
                      class="bg-green-500 h-full transition-all duration-150" 
                      :style="{ width: plugin.globalProgress + '%' }"
                    ></div>
                  </div>
                  <span class="text-xs font-mono text-gray-500">{{ plugin.globalProgress }}%</span>
                </div>
              </div>
              <span v-else class="text-gray-400 text-xs italic">Bereit</span>
            </td>

            <td class="p-4 text-right">
              <button
                class="bg-blue-600 text-white px-4 py-1.5 rounded-md text-sm font-medium hover:bg-blue-700 disabled:bg-gray-200 disabled:text-gray-400 transition-colors"
                :disabled="plugin.isGlobalRunning"
                @click="runGlobal(plugin.id)"
              >
                Auf alle anwenden
              </button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>