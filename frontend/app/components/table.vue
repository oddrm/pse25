<template>
  <div class="flex flex-col items-center">
    <input placeholder="Search" class="input" />
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
        <Entry v-for="entry in entries" :key="entry.entryID" v-bind="entry" @select="$emit('select', $event)" />
      </tbody>
    </table>
  </div>
</template>

<script setup lang="ts">
import { Sorting, columns as ENTRY_COLUMNS } from "~/utils/entryColumns";

const columns = ENTRY_COLUMNS;

const searchString = ref("");
const sortBy = ref(Sorting.Name);
const ascending = ref(true);

const { data: entries, refresh: refreshEntries, error: entriesFetchError, status: entriesStatus } = await useAsyncData("entries", async () => fetchEntries(searchString.value, sortBy.value, ascending.value, 1, 50));

</script>