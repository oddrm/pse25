<template>
  <div class="p-4 rounded bg-gray-50 mr-3">
    <table class="table table-sm">
      <thead>
        <tr>
          <th class="w-40">Name</th>
          <th class="w-40">Start</th>
          <th class="w-40">End</th>
          <th>Description</th>
          <th class="w-10"></th>
        </tr>
      </thead>

      <tbody>
        <tr v-for="seq in sequences" :key="seq.id">
          <td>{{ seq.name }}</td>
          <td>{{ formatDate(seq.startTime) }}</td>
          <td>{{ formatDate(seq.endTime) }}</td>
          <td>{{ seq.description }}</td>

          <td class="text-right">
            <button
              class="text-blue-500 hover:text-blue-700"
              type="button"
              @click="deleteSequence(seq.id)"
              title="Delete"
            >
              <Icon name="solar:trash-bin-minimalistic-linear" size="18" />
            </button>
          </td>
        </tr>
      </tbody>
    </table>

    <!-- Button -->
    <div class="mt-2">
      <button class="btn btn-xs btn-primary" @click="openModal">+</button>
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
        <button class="btn btn-sm btn-primary" @click="addSequence">OK</button>
      </div>

    </div>
  </dialog>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from "vue"
import { useSequencesStore } from "../../stores/sequenceStore"

const props = defineProps<{ entryID: number }>()

const store = useSequencesStore()

onMounted(() => {
  store.init()
})

const sequences = computed(() => store.byEntry(props.entryID))

const formatDate = (date: Date | null) => {
  if (!date) return ""

  return date.toLocaleString("en-GB", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  })
}


const modalRef = ref<HTMLDialogElement | null>(null)

const initialForm = {
  name: "",
  startTime: "", // datetime-local -
  endTime: "",
  description: "",
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
  const now = new Date()
  const name = form.value.name?.trim() || "Untitled"

  const start = form.value.startTime
    ? new Date(form.value.startTime)
    : now

  const end = form.value.endTime
    ? new Date(form.value.endTime)
    : null

  store.add({
    name,
    startTime: start,
    endTime: end,
    description: form.value.description || "",
    entryID: props.entryID,
  })

  closeModal()
}

function deleteSequence(id: number) {
  store.remove(id)
}
</script>
