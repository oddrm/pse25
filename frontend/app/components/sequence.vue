<template>
  <div class="p-4 rounded bg-base-200 mr-3 border border-base-300">
    <div class="flex justify-between items-center mb-2">
      <h3 class="font-bold text-base-content">Sequences</h3>
      <button class="btn btn-xs btn-primary" @click="openModal(null)">+</button>
    </div>

    <table class="table table-sm bg-base-100 rounded-lg shadow">
      <thead class="bg-base-200">
        <tr>
          <th class="w-32">Name</th>
          <th class="w-24">Start</th>
          <th class="w-24">End</th>
          <th>Description</th>
          <th class="w-40">Tags</th>
          <th class="w-20 text-right">Actions</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="seq in sequences" :key="seq.id" class="hover group">
          <td class="font-medium">{{ seq.name }}</td>
          <td class="font-mono text-xs">{{ formatSeconds(seq.start_timestamp) }}</td>
          <td class="font-mono text-xs">{{ formatSeconds(seq.end_timestamp) }}</td>
          <td class="text-base-content/80 text-sm truncate max-w-xs">{{ seq.description }}</td>

          <td>
            <div class="flex flex-wrap gap-1">
              <span v-for="t in seq.tags" :key="t" class="badge badge-sm badge-primary border-none">
                {{ t }}
              </span>
            </div>
          </td>

          <td class="text-right flex justify-end gap-1">
            <button class="btn btn-ghost btn-xs text-warning" @click="openModal(seq)">
              <Icon name="solar:pen-new-square-linear" size="16" />
            </button>
            <button class="btn btn-ghost btn-xs text-error" @click="deleteSequence(seq.id)">
              <Icon name="solar:trash-bin-minimalistic-linear" size="16" />
            </button>
          </td>
        </tr>
        <tr v-if="sequences.length === 0">
          <td colspan="6" class="text-center text-base-content/40 text-sm py-4">No sequences available</td>
        </tr>
      </tbody>
    </table>

    <dialog ref="modalRef" class="modal">
      <div class="modal-box w-11/12 max-w-lg overflow-visible">
        <h3 class="font-bold text-lg mb-6 border-b pb-2">
          {{ isEditing ? 'Edit sequence' : 'Create new sequence' }}
        </h3>

        <div class="form-control w-full mb-4">
          <label class="label py-1"><span class="label-text font-bold">Name</span></label>
          <input class="input input-sm input-bordered w-full" placeholder="e.g. Loop Closure" v-model="formName" />
        </div>

        <div class="px-2 mb-6 mt-8">
          <label class="label py-1 mb-2"><span class="label-text font-bold label-text-alt">Select time range</span></label>

          <Slider :model-value="[currentStartTime, currentEndTime]" @slide="updateFromSlider" :min="0"
            :max="props.totalDuration > 0 ? props.totalDuration : 1" :step="1" :tooltips="false"
            class="slider-primary z-10" />

          <div class="flex justify-between text-[10px] text-base-content/40 mt-1 px-1 font-mono">
            <span>00:00</span>
            <span>{{ formatSeconds(props.totalDuration) }}</span>
          </div>
        </div>

        <div class="flex gap-4 mb-4">
          <div class="form-control w-1/2">
            <label class="label py-1"><span class="label-text font-bold">Start time</span></label>
            <div class="join w-full">
              <input type="number" min="0" class="input input-sm input-bordered join-item w-1/2 text-center font-mono"
                placeholder="Min" v-model="startMin" />
              <span class="bg-base-200 flex items-center px-1 font-bold">:</span>
              <input type="number" min="0" max="59"
                class="input input-sm input-bordered join-item w-1/2 text-center font-mono" placeholder="Sec"
                v-model="startSec" />
            </div>
          </div>

          <div class="form-control w-1/2">
            <label class="label py-1"><span class="label-text font-bold">End time</span></label>
            <div class="join w-full">
              <input type="number" min="0" class="input input-sm input-bordered join-item w-1/2 text-center font-mono"
                placeholder="Min" v-model="endMin" />
              <span class="bg-base-200 flex items-center px-1 font-bold">:</span>
              <input type="number" min="0" max="59"
                class="input input-sm input-bordered join-item w-1/2 text-center font-mono" placeholder="Sec"
                v-model="endSec" />
            </div>
          </div>
        </div>

        <div class="form-control w-full mb-4">
          <label class="label py-1"><span class="label-text font-bold">Tags</span></label>
          <TagEditor :tags="formTags" @update="(newTags) => formTags = newTags" />
        </div>

        <div class="form-control w-full mb-4">
          <label class="label py-1"><span class="label-text font-bold">Description</span></label>
          <textarea class="textarea textarea-bordered w-full h-20" v-model="formDesc"></textarea>
        </div>

        <div class="modal-action flex justify-between items-center pt-2 border-t">
          <div class="text-xs text-base-content/60 font-mono">
            Dauer: {{ formatSeconds(currentEndTime - currentStartTime) }}
          </div>
          <div class="gap-2 flex">
            <button class="btn btn-sm" @click="closeModal">Cancel</button>
            <button class="btn btn-sm btn-primary" @click="saveSequence">
              {{ isEditing ? 'Save' : 'Create' }}
            </button>
          </div>
        </div>
      </div>
    </dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue"
import { useSequencesStore } from "../../stores/sequenceStore"
import type { Sequence } from "~/utils/sequence"
import Slider from '@vueform/slider'
import TagEditor from '~/components/TagEditor.vue'
import '@vueform/slider/themes/default.css'

const props = defineProps<{
  entryID: number,
  totalDuration: number
}>()

const store = useSequencesStore()
onMounted(async () => { await store.init(); await store.loadForEntries([props.entryID]) })
watch(() => props.entryID, async (id) => { if (process.client) await store.loadForEntries([id]) })
const sequences = computed(() => store.byEntry(props.entryID))

// state
const currentStartTime = ref(0);
const currentEndTime = ref(0);
const formName = ref("");
const formDesc = ref("");
const formTags = ref<string[]>([]);

const isEditing = ref(false);
const editingId = ref<number | null>(null);
const modalRef = ref<HTMLDialogElement | null>(null);

// echtzeit Slider update
// @slide sorgt für Aktualisierung während des Ziehens
const updateFromSlider = (val: any) => {
  // Da @slide ein Array [number, number] liefert:
  currentStartTime.value = val[0];
  currentEndTime.value = val[1];
}

// computed inputs
// Verknüpfung der getrennten Input-Felder (Min/Sek) mit den zentralen Sekunden-Werten
const startMin = computed({
  get: () => Math.floor(currentStartTime.value / 60),
  set: (val) => {
    const s = currentStartTime.value % 60;
    currentStartTime.value = (Number(val) * 60) + s;
  }
});

const startSec = computed({
  get: () => Math.floor(currentStartTime.value % 60),
  set: (val) => {
    const m = Math.floor(currentStartTime.value / 60);
    currentStartTime.value = (m * 60) + Number(val);
  }
});

const endMin = computed({
  get: () => Math.floor(currentEndTime.value / 60),
  set: (val) => {
    const s = currentEndTime.value % 60;
    currentEndTime.value = (Number(val) * 60) + s;
  }
});

const endSec = computed({
  get: () => Math.floor(currentEndTime.value % 60),
  set: (val) => {
    const m = Math.floor(currentEndTime.value / 60);
    currentEndTime.value = (m * 60) + Number(val);
  }
});


// Helper Functions
const formatSeconds = (totalSeconds: number | null) => {
  if (totalSeconds == null) return "00:00";
  const m = Math.floor(totalSeconds / 60);
  const s = Math.floor(totalSeconds % 60);
  return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
}

function openModal(seq: Sequence | null) {
  if (seq) {
    isEditing.value = true;
    editingId.value = seq.id;
    currentStartTime.value = seq.start_timestamp;
    currentEndTime.value = seq.end_timestamp;
    formName.value = seq.name || "";
    formDesc.value = seq.description;
    formTags.value = [...(seq.tags || [])];
  } else {
    isEditing.value = false;
    editingId.value = null;
    currentStartTime.value = 0;
    currentEndTime.value = props.totalDuration || 60;
    formName.value = "";
    formDesc.value = "";
    formTags.value = [];
  }
  modalRef.value?.showModal();
}

function closeModal() {
  modalRef.value?.close();
}

function saveSequence() {
  let start = Math.round(Math.max(0, currentStartTime.value));
  let end = Math.round(Math.min(props.totalDuration, currentEndTime.value));
  if (start > end) start = end;

  const payload: Omit<Sequence, "id" | "created_at" | "updated_at"> = {
    name: formName.value?.trim() || "Untitled",
    start_timestamp: start,
    end_timestamp: end,
    description: formDesc.value || "",
    entry_id: props.entryID,
    tags: formTags.value
  };

  if (isEditing.value && editingId.value !== null) {
    store.update({ id: editingId.value, created_at: "", updated_at: "", ...payload } as Sequence);
  } else {
    store.add(payload);
  }
  closeModal();
}

function deleteSequence(id: number) {
  if (confirm("Sequenz löschen?")) {
    store.remove(id);
  }
}
</script>

<style scoped>
.slider-primary {
  --slider-connect-bg: hsl(var(--p));
  --slider-handle-bg: hsl(var(--p));
  --slider-handle-ring-color: hsla(var(--p) / 0.3);
}
</style>