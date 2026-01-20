pub mod entry;
pub mod metadata;
pub mod sequence;
pub mod storage_manager;

/*
TODO implement poll watcher
notify crates poll watcher rescans file system everytime,
uses walkdir, hashes whole file, also uses update time for comparison.
Probably needs modified Pollwatcher implementation. https://github.com/notify-rs/notify/blob/b912ce5400010eab383180c69df798e991e9b922/notify/src/poll.rs

// TODO use inotify/pollwatcher without content check
*/
