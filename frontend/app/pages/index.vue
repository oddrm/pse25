<template>
  <div class="flex flex-col items-center">
    <input placeholder="Search" class="input" />
    <table class="table">
      <thead>
        <tr>
          <th v-for="(column, index) in columns" :key="index">{{ column }}</th>
        </tr>
      </thead>
      <tbody>
        <Entry v-for="(entry, index) in entries" :key="entry.entryID" v-bind="entry" />
      </tbody>
    </table>
  </div>
</template>


<script setup lang="ts">

const columns = [
  "Name",
  "Path",
  "Size",
  "Platform",
  "Tags",
];

const searchString = ref("");
const sortBy = ref(entryColumn.Name);
const ascending = ref(true);

const { data: entries, refresh: refreshEntries, error: entriesFetchError, status: entriesStatus } = await useAsyncData("entries", async () => fetchEntries(searchString.value, sortBy.value, ascending.value, 1, 50));

</script>