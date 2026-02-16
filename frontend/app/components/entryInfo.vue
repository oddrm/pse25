<template>

  <div v-if="entry && editableEntry" class="h-[calc(100vh-3rem)] flex flex-col bg-base-100">

    <div class="flex-1 overflow-y-auto p-4 space-y-6 pb-24">
      <h2 class="text-xl font-bold border-b border-base-300 pb-2">INFO</h2>

      <div class="grid grid-cols-2 gap-x-4 gap-y-2 bg-base-200 p-4 rounded-lg shadow-inner">
        <div class="flex flex-col">
          <span class="text-xs font-bold opacity-50 uppercase">Name</span>
          <span class="truncate">{{ editableEntry.name }}</span>
        </div>
        <div class="flex flex-col">
          <span class="text-xs font-bold opacity-50 uppercase">Size</span>
          <span>{{ (editableEntry.size / 1000 / 1000).toFixed(2) }} MB</span>
        </div>
        <div class="flex flex-col col-span-2">
          <span class="text-xs font-bold opacity-50 uppercase">Path</span>
          <span class="break-all font-mono text-xs">{{ editableEntry.path }}</span>
        </div>

        <div class="flex flex-col col-span-2">
          <span class="text-xs font-bold opacity-50 uppercase">Platform</span>
          <input v-model="editableEntry.platform_name"
            class="input input-bordered input-xs bg-base-100 font-bold mt-1" />
        </div>

        <div class="flex flex-col">
          <span class="text-xs font-bold opacity-50 uppercase">Scenario</span>
          <input v-model="editableEntry.scenario_name" class="input input-bordered input-xs bg-base-100 mt-1" />
        </div>
        <div class="flex flex-col">
          <span class="text-xs font-bold opacity-50 uppercase">Scenario Creation Time</span>
          <input v-model="editableEntry.scenario_creation_time" type="datetime-local"
            class="input input-bordered input-xs bg-base-100 mt-1 text-[10px]" />
        </div>

        <div class="flex flex-col">
          <span class="text-xs font-bold opacity-50 uppercase">Distance</span>
          <div class="flex gap-1 items-center">
            <input v-model.number="editableEntry.sequence_distance" type="number" step="0.01"
              class="input input-bordered input-xs bg-base-100 mt-1 w-full" />
            <span class="text-xs mt-1">m</span>
          </div>
        </div>
        <div class="flex flex-col"></div>

        <div class="flex flex-col">
          <span class="text-xs font-bold opacity-50 uppercase">Lat</span>
          <input v-model.number="editableEntry.sequence_lat_starting_point_deg" type="number" step="0.000001"
            class="input input-bordered input-xs bg-base-100 mt-1" />
        </div>
        <div class="flex flex-col">
          <span class="text-xs font-bold opacity-50 uppercase">Lon</span>
          <input v-model.number="editableEntry.sequence_lon_starting_point_deg" type="number" step="0.000001"
            class="input input-bordered input-xs bg-base-100 mt-1" />
        </div>
      </div>

      <!-- Weather Section -->
      <div class="bg-base-200 p-4 rounded-lg">
        <h3 class="font-bold mb-3 text-sm opacity-70 uppercase tracking-widest">Weather</h3>
        <div class="grid grid-cols-2 gap-x-4 gap-y-2 text-sm">
          <div class="flex flex-col">
            <span class="text-[10px] font-bold opacity-50 uppercase">Cloudiness</span>
            <input v-model="editableEntry.weather_cloudiness" class="input input-bordered input-xs bg-base-100 mt-1" />
          </div>
          <div class="flex flex-col">
            <span class="text-[10px] font-bold opacity-50 uppercase">Precipitation</span>
            <input v-model="editableEntry.weather_precipitation"
              class="input input-bordered input-xs bg-base-100 mt-1" />
          </div>
          <div class="flex flex-col">
            <span class="text-[10px] font-bold opacity-50 uppercase">Deposits</span>
            <input v-model="editableEntry.weather_precipitation_deposits"
              class="input input-bordered input-xs bg-base-100 mt-1" />
          </div>
          <div class="flex flex-col">
            <span class="text-[10px] font-bold opacity-50 uppercase">Wind</span>
            <input v-model="editableEntry.weather_wind_intensity"
              class="input input-bordered input-xs bg-base-100 mt-1" />
          </div>
          <div class="flex flex-col">
            <span class="text-[10px] font-bold opacity-50 uppercase">Humidity</span>
            <input v-model="editableEntry.weather_road_humidity"
              class="input input-bordered input-xs bg-base-100 mt-1" />
          </div>
          <div class="flex flex-col">
            <span class="text-[10px] font-bold opacity-50 uppercase">Flags</span>
            <div class="flex gap-4 mt-2">
              <label class="flex items-center gap-2 cursor-pointer">
                <input type="checkbox" v-model="editableEntry.weather_fog" class="toggle toggle-primary toggle-xs" />
                <span class="text-xs">Fog</span>
              </label>
              <label class="flex items-center gap-2 cursor-pointer">
                <input type="checkbox" v-model="editableEntry.weather_snow" class="toggle toggle-primary toggle-xs" />
                <span class="text-xs">Snow</span>
              </label>
            </div>
          </div>
        </div>
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
              <span>Type: {{ topic.topic_type }}</span>
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
          placeholder="Description of the bagfile..."></textarea>
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
              <label class="label py-0"><span class="label-text-alt opacity-60">Type</span></label>
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

                <select class="select select-bordered select-xs" @change="(e) => {
                  const target = e.target as HTMLSelectElement;
                  if (target.value && target.value !== 'default') {
                    addTopicToSensor(sensor, target.value);
                    target.value = 'default';
                  }
                }">
                  <option value="default" disabled selected>+ Topic</option>
                  <option v-for="availableTopic in getFilteredTopics(sensor)" :key="availableTopic.topic_name"
                    :value="availableTopic.topic_name">
                    {{ availableTopic.topic_name }}
                  </option>
                </select>
              </div>

              <div v-if="!sensor.ros_topics || sensor.ros_topics.length === 0"
                class="text-[10px] italic opacity-40 mt-1">
                No topics assigned
              </div>
            </div>
          </div>
        </div>

        <div v-if="showSensorSelect"
          class="bg-base-100 p-3 rounded-lg border-2 border-dashed border-base-300 space-y-3">
          <select v-model="selectedExistingSensor" class="select select-bordered select-sm w-full">
            <option :value="null" disabled>Select existing sensor</option>
            <option v-for="s in globalSensors" :key="s.sensor_name" :value="s">{{ s.sensor_name }} ({{ s.sensor_type }})
            </option>
          </select>
          <div class="flex gap-2">
            <button @click="addExistingSensor" class="btn btn-secondary btn-sm flex-1"
              :disabled="!selectedExistingSensor">From list</button>
            <button @click="addNewEmptySensor" class="btn btn-accent btn-sm flex-1">Create new</button>
          </div>
        </div>

        <button v-else @click="showSensorSelect = true" class="btn btn-primary btn-sm btn-outline w-full">
          + Add sensor
        </button>
      </div>
    </div>

    <div class="p-4 bg-base-200 border-t border-base-300 flex gap-3 shadow-lg">
      <button @click="saveChanges" class="btn btn-primary flex-1" :disabled="isSaving">
        <span v-if="isSaving" class="loading loading-spinner loading-xs"></span>
        {{ isSaving ? 'Saving...' : 'Save' }}
      </button>
      <button @click="cancelChanges" class="btn btn-ghost flex-1" :disabled="isSaving">Cancel</button>

      <!-- Toast Notification -->
      <div v-if="showToast" class="toast toast-end toast-bottom z-[200]">
        <div
          :class="['alert', toastType === 'success' ? 'alert-success' : 'alert-error', 'shadow-lg', 'text-white', 'font-bold']">
          <span>{{ toastMessage }}</span>
        </div>
      </div>
    </div>
  </div>

  <div v-else class="p-8 text-center text-base-content/40 flex flex-col items-center justify-center h-full">
    <Icon name="octicon:info-24" class="w-12 h-12 mb-2 opacity-20" />
    <p>Select a file from the list.</p>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted } from 'vue'
import { Sorting } from '~/utils/entryColumns'
import type { Entry, entryID } from '~/utils/entry'
import type { Sensor, SensorWeb } from '~/utils/sensor'
import type { MetadataWeb } from '~/utils/metadata'
import { fetchEntry, updateMetadata, addSensor, fetchSensors, removeSensor as apiRemoveSensor, fetchTopics, fetchAllSensors, updateSensor } from '~/utils/dbQueries'

const props = defineProps<{
  entryID: entryID | null
}>()

const entry = ref<Entry | null>(null)
const editableEntry = ref<Entry | null>(null)
const showSensorSelect = ref(false)
const selectedExistingSensor = ref<Sensor | null>(null)
const deletedSensorIds = ref<number[]>([])
const isSaving = ref(false)
const showToast = ref(false)
const toastMessage = ref('')
const toastType = ref<'success' | 'error'>('success')

const triggerToast = (message: string, type: 'success' | 'error' = 'success') => {
  toastMessage.value = message
  toastType.value = type
  showToast.value = true
  setTimeout(() => {
    showToast.value = false
  }, 3000)
}

const loadEntryData = async (numericId: number) => {
  try {
    const e = await fetchEntry(numericId)

    // Format datetime for datetime-local input (YYYY-MM-DDTHH:mm)
    if (e.scenario_creation_time) {
      const date = new Date(e.scenario_creation_time);
      const offset = date.getTimezoneOffset() * 60000;
      e.scenario_creation_time = new Date(date.getTime() - offset).toISOString().slice(0, 16);
    }

    entry.value = e
    editableEntry.value = JSON.parse(JSON.stringify(e))

    // fetch sensors and topics for this entry in parallel
    const [sensorsMap, topicsMap] = await Promise.all([
      fetchSensors(numericId).catch(err => {
        console.error('Failed to fetch sensors:', err);
        return {};
      }),
      fetchTopics(numericId).catch(err => {
        console.error('Failed to fetch topics:', err);
        return {};
      })
    ]);

    const sensors = Object.values(sensorsMap);
    const topics = Object.values(topicsMap);

    editableEntry.value!.sensors = sensors;
    entry.value!.sensors = JSON.parse(JSON.stringify(sensors));

    editableEntry.value!.topics = topics;
    entry.value!.topics = JSON.parse(JSON.stringify(topics));

  } catch (err) {
    console.error("Error fetching entry:", err)
    entry.value = null
    editableEntry.value = null
  }
}

const globalSensors = ref<Sensor[]>([])

onMounted(async () => {
  try {
    const sensorsMap = await fetchAllSensors()
    globalSensors.value = Object.values(sensorsMap)
  } catch (e) {
    console.error('error fetching global sensors', e)
  }
})

watch(
  () => props.entryID,
  async (id) => {
    if (!id) {
      entry.value = null;
      editableEntry.value = null;
      deletedSensorIds.value = [];
      return;
    }
    const numericId = Number(id);
    deletedSensorIds.value = [];
    await loadEntryData(numericId);
  },
  { immediate: true }
)

const addNewEmptySensor = () => {
  if (!editableEntry.value) return;

  const newSensor: Sensor = {
    id: 0,
    entry_id: Number(props.entryID),
    sensor_name: 'New Sensor',
    sensor_type: 'TBD',
    manufacturer: null,
    ros_topics: [],
    custom_parameters: null
  };

  if (!editableEntry.value.sensors) editableEntry.value.sensors = [];
  editableEntry.value.sensors.push(newSensor);
  showSensorSelect.value = false;
};

const addExistingSensor = () => {
  if (!editableEntry.value || !selectedExistingSensor.value) return;

  const newSensor: Sensor = {
    id: 0,
    entry_id: Number(props.entryID),
    sensor_name: selectedExistingSensor.value.sensor_name,
    manufacturer: selectedExistingSensor.value.manufacturer,
    sensor_type: selectedExistingSensor.value.sensor_type,
    ros_topics: [...selectedExistingSensor.value.ros_topics],
    custom_parameters: selectedExistingSensor.value.custom_parameters ? JSON.parse(JSON.stringify(selectedExistingSensor.value.custom_parameters)) : null
  };

  if (!editableEntry.value.sensors) editableEntry.value.sensors = [];
  editableEntry.value.sensors.push(newSensor);
  showSensorSelect.value = false;
  selectedExistingSensor.value = null;
};

const removeSensor = (index: number) => {
  if (!editableEntry.value?.sensors) return;

  const sensor = editableEntry.value.sensors[index];
  if (!sensor) {
    console.warn(`Sensor at index ${index} not found for removal.`);
    return;
  }
  if (sensor.id && sensor.id > 0) {
    deletedSensorIds.value.push(sensor.id);
  }

  editableEntry.value.sensors.splice(index, 1);
};

const saveChanges = async () => {
  if (!editableEntry.value || !props.entryID) return

  isSaving.value = true
  const numericId = Number(props.entryID);

  // Handle datetime conversion if needed
  let creationTime = editableEntry.value.scenario_creation_time;
  if (creationTime && !creationTime.endsWith('Z') && !creationTime.includes('+')) {
    creationTime = new Date(creationTime).toISOString();
  }

  const payload: MetadataWeb = {
    platform_name: editableEntry.value.platform_name || undefined,
    scenario_name: editableEntry.value.scenario_name || undefined,
    scenario_creation_time: creationTime || undefined,
    scenario_description: editableEntry.value.scenario_description || undefined,
    sequence_distance: editableEntry.value.sequence_distance ?? undefined,
    sequence_lat_starting_point_deg: editableEntry.value.sequence_lat_starting_point_deg ?? undefined,
    sequence_lon_starting_point_deg: editableEntry.value.sequence_lon_starting_point_deg ?? undefined,
    weather_cloudiness: editableEntry.value.weather_cloudiness || undefined,
    weather_precipitation: editableEntry.value.weather_precipitation || undefined,
    weather_precipitation_deposits: editableEntry.value.weather_precipitation_deposits || undefined,
    weather_wind_intensity: editableEntry.value.weather_wind_intensity || undefined,
    weather_road_humidity: editableEntry.value.weather_road_humidity || undefined,
    weather_fog: editableEntry.value.weather_fog ?? undefined,
    weather_snow: editableEntry.value.weather_snow ?? undefined,
    topics: editableEntry.value.topics ? editableEntry.value.topics.map(t => t.topic_name) : undefined
  }
  try {
    // 1. Update Metadata
    await updateMetadata(numericId, payload)

    // 2. Update Sensors (if any changes were made to their names/types/topics)
    // 2a. Remove sensors that were marked for deletion
    if (deletedSensorIds.value.length > 0) {
      await Promise.all(deletedSensorIds.value.map(id => apiRemoveSensor(id)));
      deletedSensorIds.value = [];
    }

    // 2b. Add or Update sensors
    if (editableEntry.value.sensors) {
      for (const s of editableEntry.value.sensors) {
        if (s.id && s.id > 0) {
          // Update existing
          await updateSensor(numericId, s.id, {
            sensor_name: s.sensor_name,
            sensor_type: s.sensor_type,
            ros_topics: s.ros_topics,
            manufacturer: s.manufacturer,
            custom_parameters: s.custom_parameters
          });
        } else {
          // Add new (id is 0 or undefined)
          await addSensor(numericId, {
            sensor_name: s.sensor_name,
            sensor_type: s.sensor_type,
            ros_topics: s.ros_topics,
            manufacturer: s.manufacturer,
            custom_parameters: s.custom_parameters
          });
        }
      }
    }

    // Refresh everything from backend to be sure
    await loadEntryData(numericId);

    triggerToast('Changes saved successfully', 'success');
  }
  catch (e) {
    console.error('error saving changes', e);
    triggerToast('Failed to save changes', 'error');
  } finally {
    isSaving.value = false;
  }
}


const cancelChanges = () => {
  if (entry.value) {
    editableEntry.value = JSON.parse(JSON.stringify(entry.value));
    deletedSensorIds.value = [];
    triggerToast('Changes discarded', 'success')
  }
}

// Remove topic from a sensor
const removeTopicFromSensor = (sensor: Sensor, topicIndex: number) => {
  sensor.ros_topics.splice(topicIndex, 1);
};

// Add topic to a sensor
const addTopicToSensor = (sensor: Sensor, topicName: string) => {
  if (!sensor.ros_topics) sensor.ros_topics = [];
  // Only add if not already present
  if (!sensor.ros_topics.includes(topicName)) {
    sensor.ros_topics.push(topicName);
  }
  // Remove focus from select to close it
  if (document.activeElement instanceof HTMLElement) {
    document.activeElement.blur();
  }
};

// Filters the list of MCAP topics to show only those the sensor doesn't have yet.
const getFilteredTopics = (sensor: Sensor) => {
  if (!editableEntry.value?.topics) return [];
  const currentSensorTopics = sensor.ros_topics || [];
  return editableEntry.value.topics.filter(
    t => !currentSensorTopics.includes(t.topic_name)
  );
};

</script>