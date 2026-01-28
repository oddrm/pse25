<template>
  <div class="relative w-full h-screen">

    <!-- Main -->
    <div class="transition-all duration-300" :class="infoOpen ? 'w-2/3' : 'w-full'">
      <!-- Table -->
      <Table @select="openInfo" />
    </div>
    <!-- Right info curtain -->
    <div class="fixed top-0 right-0 h-full transition-all duration-300 bg-base-200 pt-12"
      :class="infoOpen ? 'w-1/3' : 'w-6'">
      <!-- Open -->
      <div v-if="infoOpen" class="h-full relative cursor-pointer">
        <button @click="infoOpen = false" class="absolute left-[-0.25rem] top-1/2 -translate-y-1/2 z-50">
          <Icon icon="octicon:triangle-right" class="w-10 h-10 text-blue-800" />
        </button>
        <EntryInfo :entryID="selectedEntryID" />
      </div>


      <!-- Closed -->
      <div v-else @click="infoOpen = true" class="absolute left-[-2.0rem] top-1/2 -translate-y-1/2 pt-12">
        <Icon icon="octicon:triangle-left" class="w-10 h-10 text-blue-800" />
      </div>
    </div>

  </div>
</template>


<script setup lang="ts">
import { Icon } from '@iconify/vue'
import Table from '~/components/table.vue'
import EntryInfo from '~/components/entryInfo.vue'
const infoOpen = ref(false)
const selectedEntryID = ref<number | null>(null)

const openInfo = (id: number) => {
  selectedEntryID.value = id
  infoOpen.value = true
}

</script>