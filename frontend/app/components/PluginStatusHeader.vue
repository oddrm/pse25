<template>
  <ClientOnly>
    <div v-if="visiblePlugins && visiblePlugins.length > 0" class="p-6 pb-0 space-y-3">
      <h2 class="text-xs font-bold text-secondary uppercase tracking-widest">Active Instances</h2>
      <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
        <div v-for="running in visiblePlugins" :key="running.runId"
          class="p-3 bg-base-200 border border-base-300 rounded-lg shadow text-sm">
          <div class="flex justify-between mb-1 font-medium items-center">
            <div>
              <span class="text-primary font-bold">{{ running.pluginName }}</span>
              <span class="opacity-50 text-[10px] mx-1">for</span>
              {{ running.entryName }}
            </div>
            <div class="flex items-center gap-2">
              <span class="text-primary font-bold">{{ running.progress }}%</span>
              <div class="flex items-center gap-2">
                <button v-if="running.state === 'Running'" class="btn btn-ghost btn-xs"
                  @click="pause(running)">Pause</button>
                <button v-else-if="running.state === 'Paused'" class="btn btn-ghost btn-xs"
                  @click="resume(running)">Resume</button>
                <button class="btn btn-error btn-xs" @click="stop(running)">Stop</button>
              </div>
            </div>
          </div>
          <progress class="progress progress-primary w-full h-1.5" :value="running.progress" max="100"></progress>
        </div>
      </div>
    </div>
  </ClientOnly>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { usePluginsStore } from '../../stores/pluginStore'

// Wir nutzen ref, damit das Template auf Änderungen reagiert
const pluginsStore = usePluginsStore()

// only show instances that are not stopped or completed
const visiblePlugins = computed(() => {
  const list = pluginsStore.runningPlugins ? pluginsStore.runningPlugins : []
  return list.filter((p: any) => p.state !== 'Stopped' && p.state !== 'Completed')
})

onMounted(() => {
  // Erst hier im Browser wird die Instanz geholt
  // ensure background polling is active so running instances are populated
  pluginsStore.startPollingRunning && pluginsStore.startPollingRunning()
})

const stop = (r: any) => {
  pluginsStore.stopInstance && pluginsStore.stopInstance(r.runId)
}

const pause = (r: any) => {
  pluginsStore.pauseInstance && pluginsStore.pauseInstance(r.runId)
}

const resume = (r: any) => {
  pluginsStore.resumeInstance && pluginsStore.resumeInstance(r.runId)
}

</script>