import type { Entry, entryID } from "./entry";
import type { Sorting } from "./entryColumns";
import type { Sequence, sequenceID } from "./sequence";

export const fetchEntries = (searchString: string, sortBy: Sorting, ascending: boolean, page: number, pageSize: number): Entry[] => {
    return [
        { 
            entryID: 1, 
            duration: 88, // WICHTIG: Dauer in Sekunden für das Sequenz-Limit
            name: "alice_excavation.mcap", 
            path: "/path/to/goose/dataset.mcap", 
            platform: "Alice", 
            size: 2048000, 
            tags: ["field", "forest", "flat", "persons", "barrels"],
            
            topics: [
                { name: "sensors/ouster_cabin_left/points", type: "sensor_msgs/PointCloud2", frequency: 10, messageCount: 300 },
                { name: "sensors/jai_fs_3200d_cabin_left/image_raw", type: "sensor_msgs/Image", frequency: 10, messageCount: 300 }
            ],
            
            description: "Alice detects barrel during excavation.",
            
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
            duration: 50, // WICHTIG
            name: "entryHaha.mcap", 
            path: "/path/to/entry.mcap", 
            platform: "Platform B", 
            size: 1000000, 
            tags: ["Tag A", "Tag B"] 
        }
    ];
};

export const fetchSequences = (entryID: entryID): Sequence[] => {
    
    // Mock-Daten für Entry 1 (Alice)
    if (entryID === 1) {
        return [
            { 
                id: 101, 
                name: "barrel_detection", 
                startTime: 10,      // Start bei Sekunde 10
                endTime: 40.1,      // Ende bei 40.1
                description: "Alice detects barrel during excavation.", 
                entryID: 1, 
                tags: ["barrel", "lidar"] // <--- NEU
            },
            { 
                id: 102, 
                name: "subsequence_grasping", 
                startTime: 50, 
                endTime: 60, 
                description: "Alice grasps barrel, while the camera fails.", 
                entryID: 1,
                tags: ["test"] // <--- NEU
            }
        ];
    }

    // Mock-Daten für Entry 2
    if (entryID === 2) {
        return [
            { 
                id: 201, 
                name: "Test Loop", 
                startTime: 0, 
                endTime: 15, 
                description: "Kurzer Test am Anfang", 
                entryID: 2,
                tags: ["test"] // <--- NEU
            }
        ];
    }

    // Standard: Leeres Array zurückgeben
    return [];
}