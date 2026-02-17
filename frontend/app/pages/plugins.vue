<script setup lang="ts">
import { onMounted } from 'vue'
import { usePluginsStore } from '../../stores/pluginStore'

const pluginsStore = usePluginsStore()

onMounted(() => {
  pluginsStore.loadPlugins()
})

// Nur den Store aufrufen, keine lokale Simulation!
const runGlobal = (id: number) => {
  pluginsStore.startPlugin(id)
}
</script>

<template>
  <div class="bg-base-100 rounded-box shadow overflow-hidden border border-base-300">
    <table class="table w-full">
      <thead>
        <tr class="bg-base-200 border-b border-base-300 text-base-content">
          <th class="p-4 text-left text-xs font-semibold uppercase tracking-wider">Plugin</th>
          <th class="p-4 text-left text-xs font-semibold uppercase tracking-wider">Beschreibung</th>
          <th class="p-4 text-left text-xs font-semibold uppercase tracking-wider">Globaler Status</th>
          <th class="p-4 text-right text-xs font-semibold uppercase tracking-wider">Aktion</th>
        </tr>
      </thead>
      <tbody class="divide-y divide-base-200">
        <tr v-for="plugin in pluginsStore.plugins" :key="plugin.id" class="hover:bg-base-50/50">
          <td class="p-4 font-medium text-base-content">{{ plugin.name }}</td>
          <td class="p-4 text-sm opacity-70">{{ plugin.description }}</td>

          <td class="p-4 w-64">
            <div v-if="plugin.isGlobalRunning">
              <div class="flex items-center gap-3">
                <progress class="progress progress-success flex-1 h-2" :value="plugin.globalProgress"
                  max="100"></progress>
                <span class="text-xs font-mono font-bold text-success">{{ plugin.globalProgress }}%</span>
              </div>
            </div>
            <span v-else class="text-base-content/40 text-xs italic">Bereit</span>
          </td>

          <td class="p-4 text-right">
            <button class="btn btn-primary btn-sm font-medium disabled:btn-ghost" :disabled="plugin.isGlobalRunning"
              @click="runGlobal(plugin.id)">
              Auf alle anwenden
            </button>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>