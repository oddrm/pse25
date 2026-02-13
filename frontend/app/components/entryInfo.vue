<template>
  <div v-if="entry" class="p-4 space-y-4 h-[calc(100vh-3rem)] overflow-y-auto pb-24">
    <h2 class="text-xl font-bold">INFO</h2>

    <div class="grid grid-cols-2 gap-2 bg-base-200 p-4 rounded-lg">
      <span class="font-bold">Name:</span> <span>{{ entry.name }}</span>
      <span class="font-bold">Path:</span> <span class="break-all">{{ entry.path }}</span>
      <span class="font-bold">Size:</span> <span>{{ entry.size }}</span>
      <span class="font-bold">Platform:</span> <span>{{ entry.platform }}</span>
    </div>

    <div class="bg-base-200 p-4 rounded-lg">
      <h3 class="font-bold mb-2">Topics</h3>
      <ul class="list-none space-y-1">
        <li v-for="topic in entry.topics" :key="topic" class="text-sm">
          > {{ topic }}
        </li>
      </ul>
    </div>

    <div class="bg-base-200 p-4 rounded-lg">
      <h3 class="font-bold mb-2">Description</h3>
      <textarea 
        v-model="entry.description" 
        class="textarea textarea-bordered w-full h-24" 
        placeholder="Informative and important Description of the File"
        @change="updateDescription"
      ></textarea>
    </div>

    <div class="bg-base-200 p-4 rounded-lg">
  <h3 class="font-bold mb-2 text-primary">Sensors</h3>
  
  <ul class="list-none space-y-2 mb-4">
  <li v-for="(sensor, index) in entry.sensors" :key="index" class="flex gap-2 items-center">
    <input v-model="sensor.name" class="input input-bordered input-sm w-full bg-base-100" placeholder="Sensor Name" />
    <input v-model="sensor.type" class="input input-bordered input-sm w-full bg-base-100" placeholder="Sensor Typ" />
    <button @click="removeSensor(index)" class="btn btn-error btn-sm btn-square" title="Löschen">
      <Icon name="octicon:trash-24" class="w-4 h-4 text-base-100" />
    </button>
  </li>
</ul>

<div v-if="showSensorSelect" class="flex flex-col gap-2">
  <select v-model="selectedExistingSensor" class="select select-bordered select-sm w-full bg-base-100">
    <option disabled :value="null">Vorhandenen Sensor wählen</option>
    <option v-for="s in globalSensors" :key="s.name" :value="s">{{ s.name }} ({{ s.type }})</option>
  </select>
  <div class="flex gap-2">
    <button @click="addExistingSensor" class="btn btn-secondary btn-sm flex-1" 
    :disabled="!selectedExistingSensor">Aus Liste wählen</button>
    <button @click="addNewEmptySensor" class="btn btn-accent btn-sm flex-1">
      Neu erstellen
    </button>
  </div>
</div>

  <button v-else @click="showSensorSelect = true" class="btn btn-primary btn-sm btn-outline mt-2">
    + Sensor hinzufügen
  </button>
</div>
  </div>

  <div v-else class="p-4 text-gray-400">
    No entry selected
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { Sorting } from '~/utils/entryColumns'
import type { Entry, entryID } from '~/utils/entry'
import { fetchEntries } from '~/utils/dbQueries'

const props = defineProps<{
  entryID: entryID | null
}>()

const entry = ref<Entry | null>(null)

watch(
  () => props.entryID,
  (id) => {
    if (!id) {
      entry.value = null
      return
    }

    const entries = fetchEntries('', Sorting.Name, true, 1, 50)
    // Erstellt eine tiefe Kopie, um direkte Mutationen des Store/Fetch-Objekts zu vermeiden
    const foundEntry = entries.find(e => e.entryID === id)
    entry.value = foundEntry ? JSON.parse(JSON.stringify(foundEntry)) : null
  },
  { immediate: true }
)

import type { Sensor } from '~/utils/entry'

const showSensorSelect = ref(false)
const selectedExistingSensor = ref<Sensor | null>(null)

const globalSensors = ref<Sensor[]>([
  { name: 'ouster_cabin_left', type: 'rotating_lidar' },
  { name: 'jai_fs_3200d_cabin_left', type: 'area_scan_camera' },
  { name: 'accurate_localization_oxford', type: 'imu' }
])

const addNewEmptySensor = () => {
  if (!entry.value) return
  if (!entry.value.sensors) entry.value.sensors = []
  
  const newSensor: Sensor = { name: '', type: '' }
  entry.value.sensors.push(newSensor)
  
  // Fügt den neuen Sensor-Referenzpunkt zur globalen Liste hinzu
  // Sobald der User die Felder im UI editiert, aktualisiert sich die globale Liste mit
  globalSensors.value.push(newSensor) 
  
  resetSensorMenu()
}

const addExistingSensor = () => {
  if (!entry.value) return
  if (!entry.value.sensors) entry.value.sensors = []
  
  if (selectedExistingSensor.value) {
    entry.value.sensors.push({ ...selectedExistingSensor.value })
  }
  resetSensorMenu()
}

const removeSensor = (index: number) => {
  if (entry.value && entry.value.sensors) {
    entry.value.sensors.splice(index, 1)
  }
}

const resetSensorMenu = () => {
  showSensorSelect.value = false
  selectedExistingSensor.value = null
}

const updateDescription = async () => {
  if (!entry.value) return
  // Hier API-Call an das Backend einfügen, z.B.:
  // await saveDescriptionToDB(entry.value.entryID, entry.value.description)
  console.log('Speichere Beschreibung:', entry.value.description)
}
</script>