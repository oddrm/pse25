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

    <!-- Button -->
    <div class="mt-2">
      <button class="btn btn-xs btn-primary" @click="openModal">
      +
      </button>
    </div>

  </div>

  <dialog ref="modalRef" class="modal">
  <div class="modal-box">
    <h3 class="font-bold text-lg mb-3">New Sequence</h3>

    <input
      class="input input-sm input-bordered w-full mb-2"
      placeholder="Name"
      v-model="form.name"
    />

    <input
      type="datetime-local"
      class="input input-sm input-bordered w-full mb-2"
      v-model="form.startTime"
    />

    <input
      type="datetime-local"
      class="input input-sm input-bordered w-full mb-2"
      v-model="form.endTime"
    />

    <textarea
      class="textarea textarea-bordered w-full mb-3"
      placeholder="Description"
      v-model="form.description"
    ></textarea>

    <div class="modal-action">
      <button class="btn btn-sm" @click="closeModal">Cancel</button>
      <button class="btn btn-sm btn-primary" @click="addSequence">
        OK
      </button>
    </div>
  </div>
</dialog>

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
  console.log('Entry ID:', props.entryID)
})

const formatDate = (date: Date) => {
  return new Date(date).toLocaleString("en-GB", {
    dateStyle: "medium",
    timeStyle: "medium"
  })
}


import { ref } from 'vue'

const modalRef = ref<HTMLDialogElement | null>(null)

const initialForm = {
  name: '',
  startTime: '',
  endTime: '',
  description: ''
}

const form = ref({ ...initialForm })

function openModal() {
  form.value = { ...initialForm }
  modalRef.value?.showModal()
}

function closeModal() {
  modalRef.value?.close()
}

function addSequence() {
  allSequences.value.push({
    name: form.value.name,
    startTime: new Date(form.value.startTime),
    endTime: new Date(form.value.endTime),
    description: form.value.description,
    entryID: props.entryID   
  })


  closeModal()
}


</script>
