<template>
  <div v-if="entry && editableEntry" class="h-[calc(100vh-3rem)] flex flex-col bg-base-100">
    
    <div class="flex-1 overflow-y-auto p-4 space-y-6 pb-24">
      <h2 class="text-xl font-bold border-b border-base-300 pb-2">INFO</h2>

      <div class="grid grid-cols-2 gap-x-4 gap-y-2 bg-base-200 p-4 rounded-lg shadow-inner">
        <div class="flex flex-col"><span class="text-xs font-bold opacity-50 uppercase">Name</span><span>{{ editableEntry.name }}</span></div>
        <div class="flex flex-col"><span class="text-xs font-bold opacity-50 uppercase">Size</span><span>{{ editableEntry.size }} KB</span></div>
        <div class="flex flex-col col-span-2"><span class="text-xs font-bold opacity-50 uppercase">Path</span><span class="break-all font-mono text-xs">{{ editableEntry.path }}</span></div>
        <div class="flex flex-col"><span class="text-xs font-bold opacity-50 uppercase">Platform</span><span>{{ editableEntry.platform }}</span></div>
      </div>

      <div class="bg-base-200 p-4 rounded-lg">
        <h3 class="font-bold mb-3 flex justify-between items-center">
          Topics 
          <span class="badge badge-sm">{{ editableEntry.topics?.length || 0 }}</span>
        </h3>
        <div class="space-y-3">
          <div v-for="topic in editableEntry.topics" :key="topic.name" class="p-2 bg-base-100 rounded border border-base-300 text-sm">
            <div class="font-bold text-primary truncate">{{ topic.name }}</div>
            <div class="grid grid-cols-2 text-[12px] mt-1 opacity-70 italic">
              <span>Typ: {{ topic.type }}</span>
              <span class="text-right">Freq: {{ topic.frequency }} Hz</span>
              <span>Messages: {{ topic.messageCount }}</span>
            </div>
          </div>
        </div>
      </div>

      <div class="bg-base-200 p-4 rounded-lg">
        <h3 class="font-bold mb-2">Description</h3>
        <textarea 
          v-model="editableEntry!.description" 
          class="textarea textarea-bordered w-full h-24 bg-base-100 text-sm" 
          placeholder="Beschreibung der Bagfile..."
        ></textarea>
      </div>

      <div class="bg-base-200 p-4 rounded-lg space-y-4">
        <h3 class="font-bold text-primary">Sensors</h3>
        
        <div v-for="(sensor, index) in editableEntry.sensors" :key="index" class="card bg-base-100 shadow border border-base-300">
          <div class="card-body p-4 gap-2">
            <div class="flex justify-between items-start">
              <span class="text-[10px] font-bold opacity-50 uppercase">Sensor #{{ index + 1 }}</span>
              <button @click="removeSensor(index)" class="btn btn-ghost btn-xs text-error btn-square">
                <Icon name="octicon:trash-24" class="w-4 h-4" />
              </button>
            </div>

            <div class="form-control w-full">
              <label class="label py-0"><span class="label-text-alt opacity-60">Name</span></label>
              <input v-model="sensor.name" class="input input-bordered input-xs font-bold" />
            </div>

            <div class="form-control w-full">
              <label class="label py-0"><span class="label-text-alt opacity-60">Typ</span></label>
              <input v-model="sensor.type" class="input input-bordered input-xs" />
            </div>

            <div class="mt-2">
           <span class="text-[10px] font-bold opacity-50 uppercase">Associated Topics</span>
            <div class="flex flex-wrap gap-1 mt-1">
               <div v-for="(t, ti) in sensor.topics" :key="ti" class="badge badge-ghost badge-sm py-2">
                  {{ t }}
                  </div>
                <div v-if="!sensor.topics || sensor.topics.length === 0" class="text-[10px] italic opacity-40">
                   Keine Topics zugeordnet
                 </div>
               </div>
              </div>
          </div>
        </div>

        <div v-if="showSensorSelect" class="bg-base-100 p-3 rounded-lg border-2 border-dashed border-base-300 space-y-3">
          <select v-model="selectedExistingSensor" class="select select-bordered select-sm w-full">
            <option :value="null" disabled>Vorhandenen Sensor wählen</option>
            <option v-for="s in globalSensors" :key="s.name" :value="s">{{ s.name }} ({{ s.type }})</option>
          </select>
          <div class="flex gap-2">
            <button @click="addExistingSensor" class="btn btn-secondary btn-sm flex-1" :disabled="!selectedExistingSensor">Aus Liste</button>
            <button @click="addNewEmptySensor" class="btn btn-accent btn-sm flex-1">Neu erstellen</button>
          </div>
        </div>

        <button v-else @click="showSensorSelect = true" class="btn btn-primary btn-sm btn-outline w-full">
          + Sensor hinzufügen
        </button>
      </div>
    </div>

    <div class="p-4 bg-base-200 border-t border-base-300 flex gap-3 shadow-lg">
      <button @click="saveChanges" class="btn btn-primary flex-1">Speichern</button>
      <button @click="cancelChanges" class="btn btn-ghost flex-1">Abbrechen</button>
    </div>
  </div>

  <div v-else class="p-8 text-center text-base-content/40 flex flex-col items-center justify-center h-full">
    <Icon name="octicon:info-24" class="w-12 h-12 mb-2 opacity-20" />
    <p>Wählen Sie eine Datei aus der Liste aus.</p>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { Sorting } from '~/utils/entryColumns'
import type { Entry, entryID, Sensor } from '~/utils/entry'
import { fetchEntries } from '~/utils/dbQueries'

const props = defineProps<{
  entryID: entryID | null
}>()

const entry = ref<Entry | null>(null)
const editableEntry = ref<Entry | null>(null)
const showSensorSelect = ref(false)
const selectedExistingSensor = ref<Sensor | null>(null)

const globalSensors = ref<Sensor[]>([
  { name: 'ouster_cabin_left', type: 'rotating_lidar', topics: ['sensors/ouster_cabin_left/points'] },
  { name: 'jai_fs_3200d_cabin_left', type: 'area_scan_camera', topics: ['sensors/jai_fs_3200d_cabin_left/image_raw'] },
  { name: 'accurate_localization_oxford', type: 'imu', topics: ['sensors/accurate_localization_oxford/nav_sat_fix'] }
])

watch(
  () => props.entryID,
  (id) => {
    if (!id) {
      entry.value = null;
      editableEntry.value = null;
      return;
    }
    const entries = fetchEntries('', Sorting.Name, true, 1, 50);
    const foundEntry = entries.find(e => e.entryID === id);
    if (foundEntry) {
      entry.value = foundEntry;
      editableEntry.value = JSON.parse(JSON.stringify(foundEntry));
    }
  },
  { immediate: true }
)

const addNewEmptySensor = () => {
  if (!editableEntry.value) return;
  if (!editableEntry.value.sensors) editableEntry.value.sensors = [];
  const newSensor: Sensor = { name: 'New Sensor', type: 'TBD', topics: [] };
  editableEntry.value.sensors.push(newSensor);
  globalSensors.value.push(newSensor); 
  showSensorSelect.value = false;
}

const addExistingSensor = () => {
  if (!editableEntry.value || !selectedExistingSensor.value) return;
  if (!editableEntry.value.sensors) editableEntry.value.sensors = [];
  editableEntry.value.sensors.push(JSON.parse(JSON.stringify(selectedExistingSensor.value)));
  showSensorSelect.value = false;
  selectedExistingSensor.value = null;
}

const removeSensor = (index: number) => {
  editableEntry.value?.sensors?.splice(index, 1);
}

const saveChanges = () => {
  if (editableEntry.value) {
    entry.value = JSON.parse(JSON.stringify(editableEntry.value));
  }
}

const cancelChanges = () => {
  if (entry.value) {
    editableEntry.value = JSON.parse(JSON.stringify(entry.value));
  }
}
</script>