import type { Entry, entryID } from "./entry";
import type { Sorting } from "./entryColumns";
import type { Sequence, sequenceID, SequenceWeb } from "./sequence";
import type { MetadataWeb } from "./metadata";
import type { Sensor, SensorWeb } from "./sensor";
import type { Topic } from "./topic";

export const fetchEntries = async (searchString: string, sortBy: Sorting, ascending: boolean, page: number, pageSize: number): Promise<Entry[]> => {
    //return [{ entryID: 1, name: "entryHihi.mcap", path: "/path/to/entry.mcap", platform: "Platform A", size: 1000000, tags: ["Tag A", "Tag B"] },
    //{ entryID: 2, name: "entryHaha.mcap", path: "/path/to/entry.mcap", platform: "Platform B", size: 1000000, tags: ["Tag A", "Tag B"] }];
    let { data, error } = await useFetch<Entry[]>("/backend/entries", {
        method: "GET",
        query: {
            search_string: searchString,
            sort_by: sortBy.toString(),
            ascending: ascending.toString(),
            page: page,
            page_size: pageSize
        }
    })
    if (!data.value) {
        if (error.value) {
            throw new Error(data.value);
        } else {
            throw new Error("unknown error");
        }
    } else {
        console.log("Fetched entries:", data.value);
        return data.value;
    }
};

export const fetchSequences = async (entryID: entryID): Promise<Record<number, Sequence>> => {
    //#[get("/backend/entries/<entry_id>/sequences")]
    let { data, error } = await useFetch<Record<sequenceID, Sequence>>("/backend/entries/" + entryID + "/sequences/tx/0")
    if (!data.value) {
        if (error.value) {
            throw new Error(error.value.message);
        } else {
            throw new Error("unknown error");
        }
    } else {
        return data.value;
    }
}

export const fetchSensors = async (entryID: entryID): Promise<Record<number, Sensor>> => {
    let { data, error } = await useFetch<Record<number, Sensor>>(`/backend/entries/${entryID}/sensors/tx/0`)
    if (!data.value) {
        if (error.value) throw new Error(error.value.message || 'error fetching sensors')
        throw new Error('unknown error')
    }
    return data.value
}

export const fetchTopics = async (entryID: entryID): Promise<Record<number, Topic>> => {
    let { data, error } = await useFetch<Record<number, Topic>>(`/backend/entries/${entryID}/topics/tx/0`)
    if (!data.value) {
        if (error.value) throw new Error(error.value.message || 'error fetching topics')
        throw new Error('unknown error')
    }
    return data.value
}

export const fetchEntry = async (entryID: entryID): Promise<Entry> => {
    // backend route requires a txid parameter; use 0 for latest
    let { data, error } = await useFetch<Entry>(`/backend/entries/${entryID}/tx/0`)
    if (!data.value) {
        if (error.value) {
            throw new Error(error.value.message || 'error')
        } else {
            throw new Error("unknown error");
        }
    } else {
        return data.value;
    }
}

export const addTag = async (entryID: entryID, tag: string): Promise<void> => {
    const { error } = await useFetch(`/backend/entries/${entryID}/tags/tx/0`, {
        method: 'PUT',
        body: tag
    })
    if (error.value) throw new Error(error.value.message || 'error adding tag')
}

export const removeTag = async (entryID: entryID, tag: string): Promise<void> => {
    const { error } = await useFetch(`/backend/entries/${entryID}/tags/tx/0`, {
        method: 'DELETE',
        body: tag
    })
    if (error.value) throw new Error(error.value.message || 'error removing tag')
}

export const addSequence = async (entryID: entryID, sequence: SequenceWeb): Promise<number> => {
    const { data, error } = await useFetch<number>(`/backend/entries/${entryID}/sequences/tx/0`, {
        method: 'POST',
        body: sequence
    })
    if (error.value) throw new Error(error.value.message || 'error adding sequence')
    return data.value as number
}

export const updateSequence = async (entryID: entryID, sequenceID: number, sequence: SequenceWeb): Promise<void> => {
    const { error } = await useFetch(`/backend/entries/${entryID}/sequences/${sequenceID}/tx/0`, {
        method: 'PUT',
        body: sequence
    })
    if (error.value) throw new Error(error.value.message || 'error updating sequence')
}

export const removeSequence = async (entryID: entryID, sequenceID: number): Promise<void> => {
    const { error } = await useFetch(`/backend/entries/${entryID}/sequences/${sequenceID}/tx/0`, {
        method: 'DELETE'
    })
    if (error.value) throw new Error(error.value.message || 'error removing sequence')
}

export const updateMetadata = async (entryID: entryID, metadata: MetadataWeb): Promise<void> => {
    const { error } = await useFetch(`/backend/entries/${entryID}/metadata/tx/0`, {
        method: 'PUT',
        body: metadata
    })
    if (error.value) throw new Error(error.value.message || 'error updating metadata')
}

export const addSensor = async (entryID: entryID, sensor: SensorWeb): Promise<number> => {
    const { data, error } = await useFetch<number>(`/backend/entries/${entryID}/sensors/tx/0`, {
        method: 'POST',
        body: sensor
    })
    if (error.value) throw new Error(error.value.message || 'error adding sensor')
    return data.value as number
}

export const updateSensor = async (entryID: entryID, sensorID: number, sensor: SensorWeb): Promise<void> => {
    const { error } = await useFetch(`/backend/entries/${entryID}/sensors/${sensorID}/tx/0`, {
        method: 'PUT',
        body: sensor
    })
    if (error.value) throw new Error(error.value.message || 'error updating sensor')
}

export const removeSensor = async (sensorID: number): Promise<void> => {
    const { error } = await useFetch(`/backend/sensors/${sensorID}/tx/0`, {
        method: 'DELETE'
    })
    if (error.value) throw new Error(error.value.message || 'error removing sensor')
}
