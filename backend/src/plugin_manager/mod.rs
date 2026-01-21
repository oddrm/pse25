pub mod manager;
pub mod plugin;

/*
TODO: implement plugin manager
AUSSER SICHTWEITE
the user/hooks trigger jobs
every job:
- has a set of parameters
- a plugin which executes the job
- can request/release locks on a number of entries
- lock required for modification, read but not enumerate
- has an own directory where it can store temporary files
- acquire lock -> read/write files -> release lock which also commits changes
- if locked file gets modified while job is running, job gets aborted
- communication over stdin/stdout
---
communication protocol:
- manager -> job initial info:
    - job parameters: Map<String, String>
    - temporary directory: PathBuf
- job -> manager:
    - everything in queries
    - file update: (targetPath: PathBuf, sourcePath: PathBuf)
    - file insert: (targetPath: PathBuf, sourcePath: PathBuf)
    - request lock: Vec<EntryID>
    - release lock: Vec<EntryID>
- manager -> job:
    - suspend
    - continue
    - stop
    - (kill)
*/
