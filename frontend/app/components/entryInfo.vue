<template>

  <div v-if="entry && editableEntry" class="h-[calc(100vh-3rem)] flex flex-col bg-base-100">

    <div class="flex-1 overflow-y-auto p-4 space-y-6 pb-24">
      <h2 class="text-xl font-bold border-b border-base-300 pb-2">INFO</h2>

      <div class="grid grid-cols-2 gap-x-4 gap-y-2 bg-base-200 p-4 rounded-lg shadow-inner">
        <div class="flex flex-col"><span class="text-xs font-bold opacity-50 uppercase">Name</span><span>{{
          editableEntry.name }}</span></div>
        <div class="flex flex-col"><span class="text-xs font-bold opacity-50 uppercase">Size</span><span>{{
          editableEntry.size }} KB</span></div>
        <div class="flex flex-col col-span-2"><span class="text-xs font-bold opacity-50 uppercase">Path</span><span
            class="break-all font-mono text-xs">{{ editableEntry.path }}</span></div>
        <div class="flex flex-col"><span class="text-xs font-bold opacity-50 uppercase">Platform</span><span>{{
          editableEntry.platform_name }}</span></div>
      </div>

      <div class="bg-base-200 p-4 rounded-lg">
        <h3 class="font-bold mb-3 flex justify-between items-center">
          Topics
          <span class="badge badge-sm">{{ editableEntry.topics?.length || 0 }}</span>
        </h3>
        <div class="space-y-3">
          <div v-for="topic in editableEntry.topics" :key="topic.topic_name"
            class="p-2 bg-base-100 rounded border border-base-300 text-sm">
            <div class="font-bold text-primary truncate">{{ topic.topic_name }}</div>
            <div class="grid grid-cols-2 text-[12px] mt-1 opacity-70 italic">
              <span>Typ: {{ topic.topic_type }}</span>
              <span class="text-right">Freq: {{ topic.frequency ? topic.frequency.toFixed(2) : 0 }} Hz</span>
              <span>Messages: {{ topic.message_count }}</span>
            </div>
          </div>
        </div>
      </div>

      <div class="bg-base-200 p-4 rounded-lg">
        <h3 class="font-bold mb-2">Description</h3>
        <textarea v-model="editableEntry!.scenario_description"
          class="textarea textarea-bordered w-full h-24 bg-base-100 text-sm"
          placeholder="Beschreibung der Bagfile..."></textarea>
      </div>

      <div class="bg-base-200 p-4 rounded-lg space-y-4">
        <h3 class="font-bold text-primary">Sensors</h3>

        <div v-for="(sensor, index) in editableEntry.sensors" :key="index"
          class="card bg-base-100 shadow border border-base-300">
          <div class="card-body p-4 gap-2">
            <div class="flex justify-between items-start">
              <span class="text-[10px] font-bold opacity-50 uppercase">Sensor #{{ index + 1 }}</span>
              <button @click="removeSensor(index)" class="btn btn-ghost btn-xs text-error btn-square">
                <Icon name="octicon:trash-24" class="w-4 h-4" />
              </button>
            </div>

            <div class="form-control w-full">
              <label class="label py-0"><span class="label-text-alt opacity-60">Name</span></label>
              <input v-model="sensor.sensor_name" class="input input-bordered input-xs font-bold" />
            </div>

            <div class="form-control w-full">
              <label class="label py-0"><span class="label-text-alt opacity-60">Typ</span></label>
              <input v-model="sensor.sensor_type" class="input input-bordered input-xs" />
            </div>

            <div class="mt-4">
              <span class="text-[10px] font-bold opacity-50 uppercase tracking-wider">Associated Topics</span>

              <div class="flex flex-wrap gap-2 mt-2">
                <div v-for="(topicName, ti) in sensor.ros_topics" :key="ti"
                  class="badge badge-secondary badge-sm py-3 gap-1 pl-3">
                  <span class="max-w-[150px] truncate">{{ topicName }}</span>
                  <button @click="removeTopicFromSensor(sensor, ti)"
                    class="btn btn-ghost btn-xs btn-circle h-4 w-4 min-h-0 hover:bg-primary-focus">
                    <Icon name="solar:close-circle-bold" size="12" />
                  </button>
                </div>

                <div class="dropdown dropdown-top">
                  <label tabindex="0"
                    class="btn btn-outline btn-xs btn-circle border-dashed opacity-50 hover:opacity-100"
                    title="Topic zuordnen">
                    <Icon name="solar:add-circle-bold" size="16" />
                  </label>

                  <ul tabindex="0"
                    class="dropdown-content z-[100] menu p-2 shadow-xl bg-base-100 border border-base-300 rounded-box w-85 max-h-60 overflow-y-auto block">
                    <li class="menu-title text-[10px] ">Verfügbare Topics</li>
                    <li v-for="availableTopic in getFilteredTopics(sensor)" :key="availableTopic.topic_name">
                      <a @click="addTopicToSensor(sensor, availableTopic.topic_name)" class="text-xs py-2">
                        {{ availableTopic.topic_name }}
                      </a>
                    </li>
                    <li v-if="getFilteredTopics(sensor).length === 0" class="text-xs italic p-2 opacity-40">
                      Alle Topics zugeordnet
                    </li>
                  </ul>
                </div>
              </div>

              <div v-if="!sensor.ros_topics || sensor.ros_topics.length === 0"
                class="text-[10px] italic opacity-40 mt-1">
                Keine Topics zugeordnet
              </div>
            </div>
          </div>
        </div>

        <div v-if="showSensorSelect"
          class="bg-base-100 p-3 rounded-lg border-2 border-dashed border-base-300 space-y-3">
          <select v-model="selectedExistingSensor" class="select select-bordered select-sm w-full">
            <option :value="null" disabled>Vorhandenen Sensor wählen</option>
            <option v-for="s in globalSensors" :key="s.sensor_name" :value="s">{{ s.sensor_name }} ({{ s.sensor_type }})
            </option>
          </select>
          <div class="flex gap-2">
            <button @click="addExistingSensor" class="btn btn-secondary btn-sm flex-1"
              :disabled="!selectedExistingSensor">Aus Liste</button>
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
import type { Entry, entryID } from '~/utils/entry'
import type { Sensor, SensorWeb } from '~/utils/sensor'
import type { MetadataWeb } from '~/utils/metadata'
import { fetchEntry, updateMetadata, addSensor, fetchSensors, removeSensor as apiRemoveSensor, fetchTopics } from '~/utils/dbQueries'

const props = defineProps<{
  entryID: entryID | null
}>()

const entry = ref<Entry | null>(null)
const editableEntry = ref<Entry | null>(null)
const showSensorSelect = ref(false)
const selectedExistingSensor = ref<Sensor | null>(null)

//TODO: HIer noch eine Funktion machen um auf alle Sensoren zuzugreifen, die liegen dann ja irgendwo im Backend,
//deshalb ist das hier eine händische Liste
const globalSensors = ref<Sensor[]>([
])

watch(
  () => props.entryID,
  async (id) => {
    if (!id) {
      entry.value = null;
      editableEntry.value = null;
      return;
    }
    try {
      const e = await fetchEntry(id)
      entry.value = e
      editableEntry.value = JSON.parse(JSON.stringify(e))
      // fetch sensors for this entry
      try {
        const sensorsMap = await fetchSensors(id)
        editableEntry.value!.sensors = Object.values(sensorsMap)
      } catch (e) {
        console.debug('no sensors or failed to fetch sensors', e)
      }
      try {
        const topicsMap = await fetchTopics(id)
        editableEntry.value!.topics = Object.values(topicsMap)
      } catch (e) {
        console.debug('no topics or failed to fetch topics', e)
      }
    } catch (err) {
      console.log("Error fetching entry:", err)
      entry.value = null
      editableEntry.value = null
    }
  },
  { immediate: true }
)

const addNewEmptySensor = () => {
  if (!editableEntry.value || !props.entryID) return;
  const newSensor: SensorWeb = { sensor_name: 'New Sensor', sensor_type: 'TBD', ros_topics: [], manufacturer: null, custom_parameters: null };
  // create on backend
  addSensor(props.entryID, newSensor)
    .then(() => fetchSensors(props.entryID as number))
    .then((map) => {
      // set sensors from backend response
      editableEntry.value!.sensors = Object.values(map)
    })
    .catch((e) => console.error('error adding sensor', e))
    .finally(() => {
      showSensorSelect.value = false;
    })
}

const addExistingSensor = () => {
  if (!editableEntry.value || !selectedExistingSensor.value || !props.entryID) return;
  addSensor(props.entryID, {
    sensor_name: selectedExistingSensor.value.sensor_name,
    manufacturer: selectedExistingSensor.value.manufacturer,
    sensor_type: selectedExistingSensor.value.sensor_type,
    ros_topics: selectedExistingSensor.value.ros_topics,
    custom_parameters: selectedExistingSensor.value.custom_parameters
  })
    .then(() => fetchSensors(props.entryID as number))
    .then((map) => {
      editableEntry.value!.sensors = Object.values(map)
    })
    .catch((e) => console.error('error adding existing sensor', e))
    .finally(() => {
      showSensorSelect.value = false;
      selectedExistingSensor.value = null;
    })
}

const removeSensor = (index: number) => {
  const sensor = editableEntry.value?.sensors?.[index]
  if (!sensor) return
  if (sensor.id) {
    apiRemoveSensor(sensor.id)
      .then(() => fetchSensors(props.entryID as number))
      .then((map) => {
        if (editableEntry.value) editableEntry.value.sensors = Object.values(map)
      })
      .catch((e) => console.error('error removing sensor', e))
  } else {
    editableEntry.value?.sensors?.splice(index, 1)
  }
}

const saveChanges = async () => {
  if (!editableEntry.value) return
  const payload: MetadataWeb = {
    scenario_description: editableEntry.value.scenario_description || undefined,
    topics: editableEntry.value.topics ? editableEntry.value.topics.map(t => t.topic_name) : undefined
  }
  try {
    await updateMetadata(editableEntry.value.id, payload)
    entry.value = JSON.parse(JSON.stringify(editableEntry.value))
  } catch (err) {
    console.error('Error saving metadata:', err)
  }
}

const cancelChanges = () => {
  if (entry.value) {
    editableEntry.value = JSON.parse(JSON.stringify(entry.value));
  }
}

// Topic von einem Sensor entfernen
const removeTopicFromSensor = (sensor: Sensor, topicIndex: number) => {
  sensor.ros_topics.splice(topicIndex, 1);
};

//Topic zu einem Sensor hinzufügen
const addTopicToSensor = (sensor: Sensor, topicName: string) => {
  if (!sensor.ros_topics) sensor.ros_topics = [];
  // Nur hinzufügen, wenn noch nicht vorhanden
  if (!sensor.ros_topics.includes(topicName)) {
    sensor.ros_topics.push(topicName);
  }
  // Fokus vom Dropdown entfernen, um es zu schließen
  if (document.activeElement instanceof HTMLElement) {
    document.activeElement.blur();
  }
};

// Filtert die Liste der MCAP-Topics,
// damit nur die angezeigt werden, die der Sensor noch nicht hat.
const getFilteredTopics = (sensor: Sensor) => {
  if (!editableEntry.value?.topics) return [];
  const currentSensorTopics = sensor.ros_topics || [];
  return editableEntry.value.topics.filter(
    t => !currentSensorTopics.includes(t.topic_name)
  );
};

</script>