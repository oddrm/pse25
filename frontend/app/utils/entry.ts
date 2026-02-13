
export interface Entry {
    name: string;
    path: string;
    // KB
    size: number;
    platform: string;
    tags: string[];
    entryID: entryID;
    topics?: string[];
    description?: string;
    sensors?: Sensor[];
}

export interface Sensor {
    name: string;
    type: string;
}

export type entryID = number;