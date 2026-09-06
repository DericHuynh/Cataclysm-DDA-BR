use cdda_core_types::progress::ReportLevel;
use cdda_data::Loader;

#[test]
fn reported_loading_reads_once_and_repeated_runs_do_not_duplicate_definitions() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("item.json"),
        r#"[{"type":"GENERIC","id":"stone","name":"stone","weight":"1 g","volume":"1 ml"}]"#,
    )
    .unwrap();
    let mut loader = Loader::new(vec![dir.path().into()]);
    for _ in 0..2 {
        let mut reports = Vec::new();
        let registry = loader.load_reported(|event| reports.push(event)).unwrap();
        assert_eq!(registry.items.len(), 1);
        assert_eq!(loader.raw_by_type()["ITEM"].len(), 1);
        assert_eq!(
            reports
                .iter()
                .filter(|r| r.stage == "Reading and parsing JSON")
                .count(),
            1
        );
        assert!(reports
            .iter()
            .any(|r| r.stage == "Resolving and converting definitions"));
        assert!(reports.iter().all(|r| r.level != ReportLevel::Error));
    }
}

#[test]
fn malformed_json_reports_source_and_never_claims_success() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("broken.json");
    std::fs::write(&path, "[broken").unwrap();
    let mut events = Vec::new();
    assert!(Loader::new(vec![dir.path().into()])
        .load_reported(|r| events.push(r))
        .is_err());
    assert!(events
        .iter()
        .any(|r| r.level == ReportLevel::Error && r.message.contains("broken.json")));
    assert!(!events.iter().any(|r| r.stage == "Definitions resolved"));
}

#[test]
fn omitted_categories_and_empty_directories_are_reported() {
    let dir = tempfile::tempdir().unwrap();
    assert!(Loader::new(vec![dir.path().into()])
        .load_reported(|_| {})
        .is_err());
    std::fs::write(
        dir.path().join("unknown.json"),
        r#"[{"type":"FUTURE_KIND","id":"test"}]"#,
    )
    .unwrap();
    let mut events = Vec::new();
    Loader::new(vec![dir.path().into()])
        .load_reported(|r| events.push(r))
        .unwrap();
    assert!(events
        .iter()
        .any(|r| r.level == ReportLevel::Warning && r.message.contains("FUTURE_KIND")));
}
