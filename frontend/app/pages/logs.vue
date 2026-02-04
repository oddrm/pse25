<template>
  <div class="p-6">
    <h1 class="text-xl font-bold mb-4">Logs</h1>

    <!-- Liste der Logs -->
    <div v-for="log in logs" :key="log.id" class="mb-2">
      <!-- Info Log -->
      <div v-if="log.type === 'info'" class="alert custom-info border border-blue-500">
        [{{ log.time }}] {{ log.message }}
      </div>

      <!-- Warn Log -->
      <div v-else-if="log.type === 'warn'" class="alert custom-warning border border-yellow-600">
        [{{ log.time }}] {{ log.message }}
      </div>

      <!-- Error Log -->
      <div v-else-if="log.type === 'error'" class="alert custom-error border border-red-500">
        [{{ log.time }}] {{ log.message }}
      </div>
    </div>
  </div>
</template>


<script setup lang="ts">
import { useLogsStore } from '../../stores/logsStore'

const logsStore = useLogsStore()
const logs = logsStore.logs
</script>


<style scoped>
/* Warn-Log angepasst für besseren Kontrast */
.custom-info {
  background-color: #bee2ff9f;
  color: #000000;
}

.custom-warning {
  background-color: #f8eec8a1;
  color: #000000;
}

.custom-error {
  background-color: #fababaa1;
  color: #000000;
}
</style>
