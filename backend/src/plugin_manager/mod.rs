/// Enthält die zentrale Verwaltungslogik für Plugins:
/// Registrierung, Start/Stop, Statusverwaltung und Event-Auslösung.
pub mod manager;

/// Definiert die grundlegenden Plugin-Datentypen wie `Plugin`,
/// `Trigger`, `TriggerKind` und `BackendEvent`.
pub mod plugin;

/// Brücke zwischen Rust und Python:
/// Import, Validierung und Auslesen von Plugin-Metadaten.
pub mod python_bridge;
