export const fetchEntries = (searchString: string, sortBy: Sorting, ascending: boolean, page: number, pageSize: number): Entry[] => {
    return [{ entryID: 1, name: "entry.mcap", path: "/path/to/entry.mcap", platform: "Platform A", size: 1000000, tags: ["Tag A", "Tag B"] }];
};

export const fetchSequences = (entryID: entryID): Map<sequenceID, Sequence> => {
    return new Map;
}
