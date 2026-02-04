<template>
  <div class="min-h-screen flex flex-col">

    <!-- Header -->
    <header class="w-full h-10 bg-base-200 flex items-center px-6 shadow fixed top-0 left-0 z-50">

      <!-- Title -->
      <div class="font-bold text-lg mr-10">
        ROSBag Database Manager
      </div>

      <!-- Navigation -->
      <nav class="flex gap-6">
        <NuxtLink
          to="/"
          class="tab"
          :class="{ 'tab-active': route.path === '/' }"
        >
          Table
        </NuxtLink>

        <NuxtLink
          to="/plugins"
          class="tab"
          :class="{ 'tab-active': route.path === '/plugins' }"
        >
          Plugins
        </NuxtLink>

        <NuxtLink
          to="/logs"
          class="tab relative"
          :class="{ 'tab-active': route.path === '/logs' }"
          @click="newLogsCount = 0"
        >
          Logs
          <span
            v-if="newLogsCount > 0 && route.path !== '/logs'"
            class="absolute top-0 right-0 w-2 h-2 rounded-full bg-red-600"
  >       </span>
      </NuxtLink>
      </nav>

    </header>

    <!-- Content -->
    <main class="flex-1 pt-12">
      <slot />
    </main>

  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useLogsStore } from '../../stores/logsStore'

const route = useRoute()
const logsStore = useLogsStore()
const newLogsCount = ref(0)

watch(
  () => logsStore.logs.length,
  (newLen, oldLen) => {
    if (route.path !== '/logs' && newLen > oldLen) newLogsCount.value++
  }
)
</script>
