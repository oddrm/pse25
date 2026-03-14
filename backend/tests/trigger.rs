//! Trigger tests use only the existing public API: BackendEvent, Plugin, Trigger, PluginManager.
//! No reliance on parse_trigger_public or other test-only system code.

#[cfg(test)]
mod tests {
    use backend::plugin_manager::manager::PluginManager;
    use backend::plugin_manager::plugin::{BackendEvent, Plugin, Trigger, TriggerKind};
    use std::path::PathBuf;

    #[test]
    fn backend_event_trigger_kind_maps_correctly() {
        assert_eq!(
            BackendEvent::EntryCreated { path: "x".to_string() }.trigger_kind(),
            Some(TriggerKind::OnEntryCreate)
        );
        assert_eq!(
            BackendEvent::EntryUpdated { path: "x".to_string() }.trigger_kind(),
            Some(TriggerKind::OnEntryUpdate)
        );
        assert_eq!(
            BackendEvent::EntryDeleted { path: "x".to_string() }.trigger_kind(),
            Some(TriggerKind::OnEntryDelete)
        );
        assert_eq!(
            BackendEvent::Manual { plugin_name: "p".to_string() }.trigger_kind(),
            None
        );
    }

    #[test]
    fn prepare_fire_event_selects_only_enabled_valid_matching_plugins() {
        let mut pm = PluginManager::new();

        let mut p1 = Plugin::new(
            "p1".to_string(),
            "d".to_string(),
            Trigger::OnEntryUpdate,
            PathBuf::from("C:/tmp/p1.py"),
        );
        p1.set_enabled(true);
        pm.registered.push(p1);

        pm.registered.push(Plugin::new(
            "p2".to_string(),
            "d".to_string(),
            Trigger::OnEntryCreate,
            PathBuf::from("C:/tmp/p2.py"),
        ));

        let mut p3 = Plugin::new(
            "p3".to_string(),
            "d".to_string(),
            Trigger::OnEntryUpdate,
            PathBuf::from("C:/tmp/p3.py"),
        );
        p3.set_enabled(false);
        pm.registered.push(p3);

        let mut p4 = Plugin::new(
            "p4".to_string(),
            "d".to_string(),
            Trigger::OnEntryUpdate,
            PathBuf::from("C:/tmp/p4.py"),
        );
        p4.set_valid(false);
        pm.registered.push(p4);

        let plans = pm
            .prepare_fire_event(&BackendEvent::EntryUpdated {
                path: "/data/x".to_string(),
            })
            .unwrap();

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].0, 0);
        assert_eq!(plans[0].1, PathBuf::from("C:/tmp/p1.py"));
    }

    #[test]
    fn prepare_fire_event_selects_on_entry_delete_plugins() {
        let mut pm = PluginManager::new();

        // matching delete plugin
        let mut del1 = Plugin::new(
            "del1".to_string(),
            "d".to_string(),
            Trigger::OnEntryDelete,
            PathBuf::from("C:/tmp/del1.py"),
        );
        del1.set_enabled(true);
        pm.registered.push(del1);

        // non-matching trigger
        pm.registered.push(Plugin::new(
            "upd1".to_string(),
            "d".to_string(),
            Trigger::OnEntryUpdate,
            PathBuf::from("C:/tmp/upd1.py"),
        ));

        // disabled matching trigger
        let mut del_disabled = Plugin::new(
            "del_disabled".to_string(),
            "d".to_string(),
            Trigger::OnEntryDelete,
            PathBuf::from("C:/tmp/del_disabled.py"),
        );
        del_disabled.set_enabled(false);
        pm.registered.push(del_disabled);

        let plans = pm
            .prepare_fire_event(&BackendEvent::EntryDeleted {
                path: "/data/deleted".to_string(),
            })
            .unwrap();

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].0, 0);
        assert_eq!(plans[0].1, PathBuf::from("C:/tmp/del1.py"));
    }
}