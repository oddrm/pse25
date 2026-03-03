<template>
  <div class="p-6 pt-14">
    <div class="flex justify-between items-center mb-6">
      <h1 class="text-2xl font-bold">Backend Logs</h1>
      <div class="flex gap-2 items-center">
        <span class="text-sm font-medium opacity-70">Level:</span>
        <select id="level-filter" v-model="levelFilter" @change="updateFilter"
          class="select select-bordered select-sm min-w-32">
          <option value="ERROR">Error</option>
          <option value="WARN">Warn & Above</option>
          <option value="INFO">Info & Above</option>
          <option value="DEBUG">Debug & Above</option>
        </select>
        <span class="text-sm font-medium opacity-70 ml-2">Limit:</span>
        <select id="log-limit" v-model="limit" @change="updateLimit" class="select select-bordered select-sm">
          <option :value="50">50</option>
          <option :value="100">100</option>
          <option :value="200">200</option>
          <option :value="500">500</option>
          <option :value="1000">1000</option>
        </select>
        <button @click="fetchLogs" class="btn btn-ghost btn-sm btn-square ml-1" :disabled="loading" title="Refresh">
          <Icon name="mdi:refresh" :class="{ 'animate-spin': loading }" class="text-lg" />
        </button>
        <span class="text-sm font-medium opacity-70 ml-4 w-max">Auto-refresh:</span>
        <input type="checkbox" class="checkbox checkbox-primary" v-model="autorefresh" />
      </div>
    </div>
  </div>

  <div v-if="error" class="alert alert-error mb-4 shadow-sm">
    <Icon name="mdi:alert-circle" />
    <span>{{ error }}</span>
  </div>

  <div class="overflow-x-auto">
    <div v-if="logs.length === 0 && !loading" class="text-center py-20 opacity-50">
      <Icon name="mdi:clipboard-text-outline" class="text-6xl mb-2" />
      <p>No log entries found.</p>
    </div>

    <div v-for="(log, index) in logs" :key="index" class="mb-1 font-mono text-[10px] sm:text-xs">
      <div :class="getLogLevelClass(log.level)" class="p-1 px-2 rounded-md flex gap-3 items-start border">
        <span class="opacity-50 shrink-0 select-none pt-0.5">{{ formatTimestamp(log.timestamp) }}</span>
        <span :class="getLevelTextClass(log.level)" class="font-bold w-12 shrink-0 text-center uppercase pt-0.5">
          {{ log.level }}
        </span>
        <span class="break-all grow whitespace-pre-wrap text-base-content">
          <span v-if="log.location" class="opacity-30 mr-2">[{{ log.location }}]</span>{{ log.message }}
        </span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useLogsStore } from '../../stores/logsStore'
import { onMounted, onUnmounted, ref } from 'vue'
import { storeToRefs } from 'pinia'

const logsStore = useLogsStore()
const { logs, loading, error, levelFilter, limit } = storeToRefs(logsStore)

const autorefresh = ref(true)

const fetchLogs = () => logsStore.fetchLogs()

const updateFilter = () => {
  logsStore.setLevelFilter(levelFilter.value)
}

const updateLimit = () => {
  logsStore.setLimit(limit.value)
}

const formatTimestamp = (ts: string) => {
  try {
    return new Date(ts).toLocaleTimeString()
  } catch {
    return ts
  }
}

const getLogLevelClass = (level: string) => {
  const l = level.toUpperCase()
  if (l.includes('ERROR')) return 'border-error bg-error/5'
  if (l.includes('WARN')) return 'border-warning bg-warning/5'
  if (l.includes('INFO')) return 'border-info bg-info/5'
  if (l.includes('DEBUG')) return 'border-base-content/20 bg-base-200/30'
  return 'border-base-200'
}

const getLevelTextClass = (level: string) => {
  const l = level.toUpperCase()
  if (l.includes('ERROR')) return 'text-error'
  if (l.includes('WARN')) return 'text-warning'
  if (l.includes('INFO')) return 'text-info'
  if (l.includes('DEBUG')) return 'text-base-content/50'
  return 'text-base-content'
}

let interval: any = null

onMounted(() => {
  fetchLogs()
  interval = setInterval(fetchLogs, 1000)
})

watch(autorefresh, (newVal) => {
  if (newVal) {
    fetchLogs()
    interval = setInterval(fetchLogs, 1000)
  } else {
    if (interval) clearInterval(interval)
  }
})

onUnmounted(() => {
  if (interval) clearInterval(interval)
})
</script>

<style scoped></style>
