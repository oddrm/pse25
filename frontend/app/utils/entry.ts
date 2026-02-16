
export interface Entry {
    name: string;
    path: string;
    // KB
    size: number;
    platform: string;
    tags: string[];
    entryID: entryID;
    // NEU: Dauer in Sekunden (wichtig für Sequenz-Max-Limit)
    duration: number;
    topics?: Topic[];
    description?: string;
    sensors?: Sensor[];
}

export interface Sensor {
    name: string;
    type: string;
    topics: string[];
}

export interface Topic {
    name: string;
    type: string;
    frequency: number;
    messageCount: number;
}

export type entryID = number;