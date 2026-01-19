<template>
  <div class="relative w-full h-screen">

    <!-- Main -->
    <div
      class="transition-all duration-300"
      :class="infoOpen ? 'w-2/3' : 'w-full'"
    >
      <!-- Table -->
      <div class="flex flex-col items-center">
        <input placeholder="Search" class="input" />
        <table class="table">
          <thead>
            <tr>
              <th v-for="(column, index) in columns" :key="index">
                {{ column }}
              </th>
            </tr>
          </thead>
          <tbody>
            <Entry
              v-for="entry in entries"
              :key="entry.entryID"
              v-bind="entry"
            />
          </tbody>
        </table>
      </div>
    </div>

    <!-- RIGHT INFO CURTAIN -->
    <div
      class="fixed top-0 right-0 h-full transition-all duration-300 bg-base-200"
      :class="infoOpen ? 'w-1/3' : 'w-6'"
    >
      <!-- OPEN -->
      <div v-if="infoOpen">
        <button @click="infoOpen = false"><Icon icon="f7:chevron-right-2" class="w-6 h-6" /></button> 
        INFORMATION
      </div>

      <!-- CLOSED -->
      <div
        v-else
        class="h-full flex items-center justify-center cursor-pointer"
        @click="infoOpen = true"
      >
        <Icon icon="f7:chevron-left-2" class="w-6 h-6" />
      </div>
    </div>

  </div>
</template>


<script setup lang="ts">
import { Icon } from '@iconify/vue'
import { Sorting } from "~/utils/entryColumns";
const infoOpen = ref(false);  

const columns = Object.keys(Sorting);

const searchString = ref("");
const sortBy = ref(Sorting.Name);
const ascending = ref(true);

const { data: entries, refresh: refreshEntries, error: entriesFetchError, status: entriesStatus } = await useAsyncData("entries", async () => fetchEntries(searchString.value, sortBy.value, ascending.value, 1, 50));

</script>