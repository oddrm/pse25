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
              <span class="text-primary font-bold">{{ progressFor(running) }}%</span>
              <div class="flex items-center gap-2">
                <button v-if="running.state === 'Running'" class="btn btn-ghost btn-xs"
                  @click="pause(running)">Pause</button>
                <button v-else-if="running.state === 'Paused'" class="btn btn-ghost btn-xs"
                  @click="resume(running)">Resume</button>
                <button class="btn btn-error btn-xs" @click="stop(running)">Stop</button>
              </div>
            </div>
          </div>
          <progress class="progress progress-primary w-full h-1.5" :value="progressFor(running)" max="100"></progress>
        </div>
      </div>
    </div>
  </ClientOnly>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { usePluginsStore } from '../../stores/pluginStore'

// Wir nutzen ref, damit das Template auf Änderungen reagiert
const pluginsStore = ref<any>(null)

// only show instances that are not stopped or completed
const visiblePlugins = computed(() => {
  const list = pluginsStore.value && pluginsStore.value.runningPlugins ? pluginsStore.value.runningPlugins : []
  return list.filter((p: any) => p.state !== 'Stopped' && p.state !== 'Completed')
})

onMounted(() => {
  // Erst hier im Browser wird die Instanz geholt
  pluginsStore.value = usePluginsStore()
  // ensure background polling is active so running instances are populated
  pluginsStore.value.startPollingRunning && pluginsStore.value.startPollingRunning()
})

const stop = (r: any) => {
  pluginsStore.value.stopInstance && pluginsStore.value.stopInstance(r.runId)
}

const pause = (r: any) => {
  pluginsStore.value.pauseInstance && pluginsStore.value.pauseInstance(r.runId)
}

const resume = (r: any) => {
  pluginsStore.value.resumeInstance && pluginsStore.value.resumeInstance(r.runId)
}

const progressFor = (r: any) => {
  const state = r && r.state ? r.state : ''
  switch (state) {
    case 'Failed':
    case 'Unresponsive':
    case 'Stopped':
      return 0
    case 'Paused':
    case 'Running':
      return 50
    case 'Completed':
      return 100
    default:
      return typeof r.progress === 'number' ? r.progress : 0
  }
}
</script>