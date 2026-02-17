<template>
  <ClientOnly>
    <div v-if="pluginsStore && pluginsStore.runningPlugins && pluginsStore.runningPlugins.length > 0" class="p-6 pb-0 space-y-3">
      <h2 class="text-xs font-bold text-secondary uppercase tracking-widest">Aktive Einzelprozesse</h2>
      <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
        <div v-for="running in pluginsStore.runningPlugins" :key="running.runId" class="p-3 bg-base-200 border border-base-300 rounded-lg shadow text-sm">
          <div class="flex justify-between mb-1 font-medium">
            <span>
              <span class="text-primary font-bold">{{ running.pluginName }}</span> 
              <span class="opacity-50 text-[10px] mx-1">FÜR</span> 
              {{ running.entryName }}
            </span>
            <span class="text-primary font-bold">{{ running.progress }}%</span>
          </div>
          <progress class="progress progress-primary w-full h-1.5" :value="running.progress" max="100"></progress>
        </div>
      </div>
    </div>
  </ClientOnly>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { usePluginsStore } from '../../stores/plugins'

// Wir nutzen ref, damit das Template auf Änderungen reagiert
const pluginsStore = ref<any>(null)

onMounted(() => {
  // Erst hier im Browser wird die Instanz geholt
  pluginsStore.value = usePluginsStore()
})
</script>