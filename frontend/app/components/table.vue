<template>
  <div class="flex flex-col items-center">
    <!-- <p>entries: {{ entries?.length }} {{ entriesFetchError }} {{ entriesStatus }} {{ entries }}</p> -->
    <input placeholder="Search" class="input" v-model="searchString" @keyup.enter="console.log(' enter on search');
    refreshEntries()" />
    <p>num pages {{ numPages }}</p>
    <table class="table table-auto">
      <thead>
        <tr>
          <th v-for="(column, index) in columns" :key="index" class="cursor-pointer select-none"
            @click="handleSort(column)">
            <div class="flex items-center justify-between">
              <span>{{ column }}</span>
              <span class="inline-block w-3 text-center">
                <span v-if="sortBy === column">{{ ascending ? '▼' : '▲' }}</span>
              </span>
            </div>
          </th>
          <th>Plugins</th>
        </tr>
      </thead>
      <tbody>
        <Entry v-for="entry in entries" :key="entry.id" v-bind="entry" expandable @select="$emit('select', $event)" />
      </tbody>
    </table>
  </div>
</template>

<script setup lang="ts">
import { Sorting, columns } from "~/utils/entryColumns";


const searchString = ref("");
const sortBy = ref(Sorting.Name);
const ascending = ref(true);

const { data: result, refresh: refreshEntries, error: entriesFetchError, status: entriesStatus } = await useAsyncData("entries", async () => fetchEntries(searchString.value, sortBy.value, ascending.value, 0, 2));

const entries = computed(() => result.value ? result.value[0] : []);
const numPages = computed(() => result.value ? result.value[1] : 0);

const handleSort = (column: string) => {
  if (
    column !== "Name" &&
    column !== "Path" &&
    column !== "Size" &&
    column !== "Platform"
  ) {
    return;
  }

  if (sortBy.value === column) {
    ascending.value = !ascending.value;
  } else {
    sortBy.value = column as Sorting;
    ascending.value = true;
  }

  refreshEntries();
};

watchEffect(() => {
  // console.log("searchString", searchString.value);
  refreshEntries()
});

</script>