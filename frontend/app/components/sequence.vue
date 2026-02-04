<template>
  <div class="p-4 rounded bg-base-100">
    <h4 class="font-semibold mb-2">Sequences</h4>

    <table class="table table-sm">
      <thead>
        <tr>
          <th>Name</th>
          <th>Start</th>
          <th>End</th>
          <th>Description</th>
        </tr>
      </thead>

      <tbody>
        <tr v-for="seq in sequences" :key="seq.name">
          <td>{{ seq.name }}</td>
          <td>{{ seq.startTime }}</td>
          <td>{{ seq.endTime }}</td>
          <td>{{ seq.description }}</td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<script setup lang="ts">
import type { Sequence } from "~/utils/sequence";

const sequences: Sequence[] = [
  {
    name: "Sequence 1",
    startTime: new Date("2026-01-01 10:00"),
    endTime: new Date("2026-01-01 10:30"),
    description: "First hihihaha test"
  },
  {
    name: "Sequence 2",
    startTime: new Date("2020-01-01 10:00"),
    endTime: new Date("2020-01-01 10:30"),
    description: "Second hihihaha test"
  }
];
</script>
