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
            
            // Korrektur: Topic-Strings zu Topic-Objekten umwandeln
            topics: [
                { name: "sensors/ouster_cabin_left/points", type: "sensor_msgs/PointCloud2", frequency: 10, messageCount: 300 },
                { name: "sensors/jai_fs_3200d_cabin_left/image_raw", type: "sensor_msgs/Image", frequency: 10, messageCount: 300 }
            ],
            
            description: "Alice detects barrel during excavation.", //
            
            // Korrektur: Sensor-Objekte um das Feld 'topics' erweitern
            sensors: [
                { 
                    name: "ouster_cabin_left", 
                    type: "rotating_lidar", 
                    topics: ["sensors/ouster_cabin_left/points"] 
                },
                { 
                    name: "jai_fs_3200d_cabin_left", 
                    type: "area_scan_camera", 
                    topics: ["sensors/jai_fs_3200d_cabin_left/image_raw"] 
                }
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
