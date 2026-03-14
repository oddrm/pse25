//! Trigger tests: BackendEvent/TriggerKind and prepare_fire_event use public API only;
//! parse_trigger tests use the existing parse_trigger_public (no system code changes).

#[cfg(test)]
mod tests {
    use backend::error::Error;
    use backend::plugin_manager::manager::{parse_trigger_public, PluginManager};
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
    fn parse_trigger_defaults_to_manual_when_none_or_manual() {
        let t = parse_trigger_public(None).unwrap();
        assert!(matches!(t, Trigger::Manual));

        let t = parse_trigger_public(Some("manual")).unwrap();
        assert!(matches!(t, Trigger::Manual));
    }

    #[test]
    fn parse_trigger_parses_entry_triggers() {
        assert!(matches!(
            parse_trigger_public(Some("on_entry_create")).unwrap(),
            Trigger::OnEntryCreate
        ));
        assert!(matches!(
            parse_trigger_public(Some("on_entry_update")).unwrap(),
            Trigger::OnEntryUpdate
        ));
        assert!(matches!(
            parse_trigger_public(Some("on_entry_delete")).unwrap(),
            Trigger::OnEntryDelete
        ));
    }

    #[test]
    fn parse_trigger_parses_on_schedule_with_5_fields_by_prefixing_seconds() {
        let t = parse_trigger_public(Some("on_schedule: */5 * * * *")).unwrap();
        match t {
            Trigger::OnSchedule(s) => assert!(s.upcoming(chrono::Utc).next().is_some()),
            _ => panic!("expected OnSchedule"),
        }
    }

    #[test]
    fn parse_trigger_parses_on_schedule_with_6_fields_as_is() {
        let t = parse_trigger_public(Some("on_schedule: */10 * * * * *")).unwrap();
        match t {
            Trigger::OnSchedule(s) => assert!(s.upcoming(chrono::Utc).next().is_some()),
            _ => panic!("expected OnSchedule"),
        }
    }

    #[test]
    fn parse_trigger_rejects_invalid_cron_expressions() {
        let err = parse_trigger_public(Some("on_schedule: not a cron")).unwrap_err();
        match err {
            Error::CustomError(msg) => assert!(msg.to_lowercase().contains("invalid cron")),
            _ => panic!("expected custom error"),
        }
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