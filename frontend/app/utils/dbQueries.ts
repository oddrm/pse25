import type { Entry, entryID } from "./entry";
import type { Sorting } from "./entryColumns";
import type { Sequence, sequenceID } from "./sequence";

export const fetchEntries = (searchString: string, sortBy: Sorting, ascending: boolean, page: number, pageSize: number): Entry[] => {
    return [
        { 
            entryID: 1, 
            name: "alice_excavation.mcap", 
            path: "/path/to/goose/dataset.mcap", 
            platform: "Alice", 
            size: 2048000, 
            tags: ["field", "forest", "flat", "persons", "barrels"],
            topics: [
                "sensors/ouster_cabin_left/points", 
                "sensors/ouster_cabin_left/nav_sat_fix",
                "sensors/jai_fs_3200d_cabin_left/image_raw",
                "sensors/jai_fs_3200d_cabin_left/camera_info",
                "sensors/accurate_localization_oxford/nav_sat_fix"
            ],
            description: "Alice detects barrel during excavation.",
            sensors: [
                { name: "ouster_cabin_left", type: "rotating_lidar" }, 
                { name: "jai_fs_3200d_cabin_left", type: "area_scan_camera" },
                { name: "accurate_localization_oxford", type: "imu" }
            ]
        },
        { 
            entryID: 2, 
            name: "entryHaha.mcap", 
            path: "/path/to/entry.mcap", 
            platform: "Platform B", 
            size: 1000000, 
            tags: ["Tag A", "Tag B"] 
        }
    ];
};

export const fetchSequences = (entryID: entryID): Map<sequenceID, Sequence> => {
    return new Map;
}
