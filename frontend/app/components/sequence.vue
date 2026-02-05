<template>
  <div class="p-4 rounded bg-gray-50 mr-3">

    <table class="table table-sm">
      <thead>
        <tr>
          <th class="w-40">Name</th>
          <th class="w-40">Start</th>
          <th class="w-40">End</th>
          <th>Description</th>
        </tr>
      </thead>

      <tbody>
        <tr v-for="seq in sequences" :key="seq.name">
          <td>{{ seq.name }}</td>
          <td>{{ formatDate(seq.startTime) }}</td>
          <td>{{ formatDate(seq.endTime) }}</td>
          <td>{{ seq.description }}</td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<script setup lang="ts">
import type { Sequence } from "~/utils/sequence";

  const props = defineProps<{
    entryID: number
  }>()


const allSequences = ref<Sequence[]>([
  {
    name: "Sequence 1",
    startTime: new Date("2026-01-01 10:00"),
    endTime: new Date("2026-01-01 10:30"),
    description: "First hihihaha test",
    entryID: 1
  },
  {
    name: "Sequence 1",
    startTime: new Date("2020-01-01 10:00"),
    endTime: new Date("2020-01-01 10:30"),
    description: "Second hihihaha test",
    entryID: 2
  }
])

const sequences = computed(() =>
  allSequences.value.filter(seq => seq.entryID === props.entryID)
)
watchEffect(() => {
  console.log('ENTRY ID:', props.entryID)
})

const formatDate = (date: Date) => {
  return new Date(date).toLocaleString("en-GB", {
    dateStyle: "medium",
    timeStyle: "medium"
  })
}

</script>
