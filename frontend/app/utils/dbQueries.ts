import type { Entry, entryID } from "./entry";
import type { Sorting } from "./entryColumns";
import type { Sequence, sequenceID } from "./sequence";

export const fetchEntries = async (searchString: string, sortBy: Sorting, ascending: boolean, page: number, pageSize: number): Promise<Entry[]> => {
    //return [{ entryID: 1, name: "entryHihi.mcap", path: "/path/to/entry.mcap", platform: "Platform A", size: 1000000, tags: ["Tag A", "Tag B"] },
//{ entryID: 2, name: "entryHaha.mcap", path: "/path/to/entry.mcap", platform: "Platform B", size: 1000000, tags: ["Tag A", "Tag B"] }];
    let {data, error} = await useFetch<Entry[]>("/backend/entries?" + searchString + "&" + sortBy + "&"
         + ascending ? "true" : "false" + "&" + page + "&" + pageSize)
    if (!data.value) {
        if (error.value) {
            throw new Error(error.value.message);
        } else {
            throw new Error("unknown error");
        }
    } else {
        return data.value;
    }
};

export const fetchSequences = async (entryID: entryID): Promise<Map<number, Sequence>> => {
    //#[get("/backend/entries/<entry_id>/sequences")]
    let {data, error} = await useFetch<Map<sequenceID, Sequence>>("/backend/entries/"  + entryID + "/sequences")
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
