<template>
  <div class="flex flex-col items-center grow min-h-screen pt-10">
    <!-- <p>entries: {{ entries?.length }} {{ entriesFetchError }} {{ entriesStatus }} {{ entries }}</p> -->
    <input placeholder="Search" class="input mb-10 mt-4" v-model="searchString" @keyup.enter="console.log(' enter on search');
    refreshEntries()" />
    <div class="grow w-full px-10">
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
    <!-- Pagination -->
    <div class="w-full flex justify-center pb-6 pt-2">
      <div class="join" v-if="numPages > 1">
        <button v-for="p in pagesToShow" :key="String(p)" class="join-item btn btn-sm"
          :class="p === page ? 'btn-active' : ''" :disabled="p === '...'" @click="typeof p === 'number' && goTo(p)">
          {{ p }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { Sorting, columns } from "~/utils/entryColumns";
import { useSequencesStore } from "../../stores/sequenceStore";
import { watch } from "vue";

const page = ref(1);
const pageSize = ref(4);
const offset = computed(() => (page.value - 1) * pageSize.value);


type PageToken = number | "...";

const pagesToShow = computed<PageToken[]>(() => {
  const tp = numPages.value;
  const p = page.value;

  // show all pages if there are few of them
  if (tp <= 7) return Array.from({ length: tp }, (_, i) => i + 1);

  // show 1...(p-1,p,p+1)...last
  const out: PageToken[] = [1];

  const left = Math.max(2, p - 1);
  const right = Math.min(tp - 1, p + 1);

  if (left > 2) out.push("...");
  for (let i = left; i <= right; i++) out.push(i);
  if (right < tp - 1) out.push("...");

  out.push(tp);
  return out;
});

const goTo = (p: number) => {
  page.value = p;
  refreshEntries();
};


const searchString = ref("");
const sortBy = ref(Sorting.Name);
const ascending = ref(true);
const { data: result, refresh: refreshEntries, error: entriesFetchError, status: entriesStatus } = await useAsyncData("entries", async () => fetchEntries(searchString.value, sortBy.value, ascending.value, page.value - 1, pageSize.value));

const entries = computed(() => result.value ? result.value[0] : []);
const numPages = computed(() => result.value ? result.value[1] : 0);

// Ensure sequences for visible entries are loaded
const sequencesStore = useSequencesStore()
watch(entries, (newEntries) => {
  if (!process.client) return
  const ids = (newEntries || []).map((e: any) => e.id)
  sequencesStore.loadForEntries(ids)
}, { immediate: true })

const handleSort = (column: string) => {
  if (
    column !== "Status" &&
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

  page.value = 1;
  refreshEntries();
};

watch(searchString, () => {
  page.value = 1;
  refreshEntries();
});

watch([sortBy, ascending, pageSize], () => {
  page.value = 1;
  refreshEntries();
});

</script>