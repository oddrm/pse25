import type { Entry, entryID } from "./entry";
import type { Sorting } from "./entryColumns";
import type { Sequence, sequenceID, SequenceWeb } from "./sequence";
import type { MetadataWeb } from "./metadata";
import type { Sensor, SensorWeb } from "./sensor";
import type { Topic } from "./topic";

export const fetchEntries = async (searchString: string, sortBy: Sorting, ascending: boolean, page: number, pageSize: number): Promise<[Entry[], number]> => {
    try {
        const data = await $fetch<[Entry[], number]>("/backend/entries", {
            method: "GET",
            query: {
                search_string: searchString,
                sort_by: sortBy.toString(),
                ascending: ascending.toString(),
                page: page,
                page_size: pageSize
            }
        });
        return data;
    } catch (error: any) {
        throw new Error(error.message || "unknown error");
    }
};

export const fetchSequences = async (entryID: entryID): Promise<Record<number, Sequence>> => {
    try {
        return await $fetch<Record<number, Sequence>>(`/backend/entries/${entryID}/sequences/tx/0`);
    } catch (error: any) {
        throw new Error(error.message || "unknown error");
    }
}

export const fetchSensors = async (entryID: entryID): Promise<Record<number, Sensor>> => {
    try {
        return await $fetch<Record<number, Sensor>>(`/backend/entries/${entryID}/sensors/tx/0`);
    } catch (error: any) {
        throw new Error(error.message || 'error fetching sensors');
    }
}

export const fetchAllSensors = async (): Promise<Record<number, Sensor>> => {
    try {
        return await $fetch<Record<number, Sensor>>(`/backend/sensors/tx/0`);
    } catch (error: any) {
        throw new Error(error.message || 'error fetching all sensors');
    }
}

export const fetchTopics = async (entryID: entryID): Promise<Record<number, Topic>> => {
    try {
        return await $fetch<Record<number, Topic>>(`/backend/entries/${entryID}/topics/tx/0`);
    } catch (error: any) {
        throw new Error(error.message || 'error fetching topics');
    }
}

export const fetchEntry = async (entryID: entryID): Promise<Entry> => {
    try {
        return await $fetch<Entry>(`/backend/entries/${entryID}/tx/0`);
    } catch (error: any) {
        throw new Error(error.message || 'error fetching entry');
    }
}

export const addTag = async (entryID: entryID, tag: string): Promise<void> => {
    try {
        await $fetch(`/backend/entries/${entryID}/tags/tx/0`, {
            method: 'PUT',
            body: tag
        });
    } catch (error: any) {
        throw new Error(error.message || 'error adding tag');
    }
}

export const removeTag = async (entryID: entryID, tag: string): Promise<void> => {
    try {
        await $fetch(`/backend/entries/${entryID}/tags/tx/0`, {
            method: 'DELETE',
            body: tag
        });
    } catch (error: any) {
        throw new Error(error.message || 'error removing tag');
    }
}

export const addSequence = async (entryID: entryID, sequence: SequenceWeb): Promise<number> => {
    try {
        return await $fetch<number>(`/backend/entries/${entryID}/sequences/tx/0`, {
            method: 'POST',
            body: sequence
        });
    } catch (error: any) {
        throw new Error(error.message || 'error adding sequence');
    }
}

export const updateSequence = async (entryID: entryID, sequenceID: number, sequence: SequenceWeb): Promise<void> => {
    try {
        await $fetch(`/backend/entries/${entryID}/sequences/${sequenceID}/tx/0`, {
            method: 'PUT',
            body: sequence
        });
    } catch (error: any) {
        throw new Error(error.message || 'error updating sequence');
    }
}

export const removeSequence = async (entryID: entryID, sequenceID: number): Promise<void> => {
    try {
        await $fetch(`/backend/entries/${entryID}/sequences/${sequenceID}/tx/0`, {
            method: 'DELETE'
        });
    } catch (error: any) {
        throw new Error(error.message || 'error removing sequence');
    }
}

export const updateMetadata = async (entryID: entryID, metadata: MetadataWeb): Promise<void> => {
    try {
        await $fetch(`/backend/entries/${entryID}/metadata/tx/0`, {
            method: 'PUT',
            body: metadata
        });
    } catch (error: any) {
        throw new Error(error.message || 'error updating metadata');
    }
}

export const addSensor = async (entryID: entryID, sensor: SensorWeb): Promise<number> => {
    try {
        return await $fetch<number>(`/backend/entries/${entryID}/sensors/tx/0`, {
            method: 'POST',
            body: sensor
        });
    } catch (error: any) {
        throw new Error(error.message || 'error adding sensor');
    }
}

export const updateSensor = async (entryID: entryID, sensorID: number, sensor: SensorWeb): Promise<void> => {
    try {
        await $fetch(`/backend/entries/${entryID}/sensors/${sensorID}/tx/0`, {
            method: 'PUT',
            body: sensor
        });
    } catch (error: any) {
        throw new Error(error.message || 'error updating sensor');
    }
}

export const removeSensor = async (sensorID: number): Promise<void> => {
    try {
        await $fetch(`/backend/sensors/${sensorID}/tx/0`, {
            method: 'DELETE'
        });
    } catch (error: any) {
        throw new Error(error.message || 'error removing sensor');
    }
}

