export interface Entry {
    name: string,
    path: string,
    // KB
    size: number,
    platform: string,
    tags: string[],
    entryID: entryID
}

export type entryID = number;