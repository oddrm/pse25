<template>
  <div class="flex flex-col items-center">
    <p>entries: {{ entries?.length }} {{ entriesFetchError }} {{ entriesStatus }}</p>
    <input placeholder="Search" class="input" v-model="searchString" @keyup.enter="console.log(' enter on search');
    refreshEntries()" />
    <table class="table">
      <thead>
        <tr>
          <th v-for="(column, index) in columns" :key="index">
            {{ column }}
          </th>
          <th>Plugins</th>
        </tr>
      </thead>
      <tbody>
        <Entry v-for="entry in entries" :key="entry.entryID" v-bind="entry" expandable
          @select="$emit('select', $event)" />
      </tbody>
    </table>
  </div>
</template>

<script setup lang="ts">
import { Sorting, columns } from "~/utils/entryColumns";


const searchString = ref("");
const sortBy = ref(Sorting.Name);
const ascending = ref(true);

const { data: entries, refresh: refreshEntries, error: entriesFetchError, status: entriesStatus } = await useAsyncData("entries", async () => fetchEntries(searchString.value, sortBy.value, ascending.value, 1, 50));

// watchEffect(() => {
//   console.log("searchString", searchString.value);
//   refreshEntries()
// });

</script>