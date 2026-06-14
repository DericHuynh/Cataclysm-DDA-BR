use cdda_intent::registry::{IntentHandler, IntentRegistry};
use cdda_intent::{parse_intent, Intent, IntentError};
use serde_json::{json, Value};

struct MoveHandler;
impl IntentHandler for MoveHandler {
    fn name(&self) -> &'static str {
        "move"
    }
    fn handle(&self, payload: &Value) -> Result<(), IntentError> {
        let (dx, dy): (i32, i32) = parse_intent!(payload, dx: i32, dy: i32).unwrap();
        assert_eq!(dx, -1);
        assert_eq!(dy, 0);
        Ok(())
    }
}

struct CancelHandler;
impl IntentHandler for CancelHandler {
    fn name(&self) -> &'static str {
        "cancel"
    }
    fn handle(&self, _payload: &Value) -> Result<(), IntentError> {
        Ok(())
    }
}

#[test]
fn new_intent_with_payload() {
    let i = Intent::new("move", json!({"dx": -1, "dy": 0}));
    assert_eq!(i.name, "move");
    assert!(i.has_payload());
    assert_eq!(i.payload, json!({"dx": -1, "dy": 0}));
}

#[test]
fn unit_intent_has_no_payload() {
    let i = Intent::unit("cancel");
    assert_eq!(i.name, "cancel");
    assert!(!i.has_payload());
}

#[test]
fn parse_intent_extracts_typed_fields() {
    let payload = json!({"dx": 1, "dy": 2, "label": "hi"});
    let (dx, dy, label): (i32, i32, String) =
        parse_intent!(payload, dx: i32, dy: i32, label: String).unwrap();
    assert_eq!((dx, dy), (1, 2));
    assert_eq!(label, "hi");
}

#[test]
fn parse_intent_missing_key_errors() {
    let payload = json!({"dx": 1});
    let res = parse_intent!(payload, dx: i32, dy: i32);
    assert_eq!(res.unwrap_err(), IntentError::KeyNotFound("dy"));
}

#[test]
fn parse_intent_type_mismatch_errors() {
    let payload = json!({"dx": "one", "dy": 0});
    let res = parse_intent!(payload, dx: i32, dy: i32);
    assert_eq!(
        res.unwrap_err(),
        IntentError::TypeMismatch {
            key: "dx",
            expected: "i32",
        }
    );
}

#[test]
fn parse_intent_non_object_errors() {
    let payload = json!([1, 2, 3]);
    let res = parse_intent!(payload, dx: i32, dy: i32);
    assert!(matches!(res.unwrap_err(), IntentError::NotAnObject(_)));
}

#[test]
fn registry_dispatches_by_name() {
    let reg = IntentRegistry::new()
        .register(MoveHandler)
        .register(CancelHandler);
    assert_eq!(reg.len(), 2);
    assert!(reg.has("move"));
    assert!(reg.has("cancel"));
    assert!(!reg.has("nope"));

    let move_intent = Intent::new("move", json!({"dx": -1, "dy": 0}));
    let cancel_intent = Intent::unit("cancel");
    assert!(reg.dispatch(&move_intent).is_ok());
    assert!(reg.dispatch(&cancel_intent).is_ok());
}

#[test]
fn registry_unknown_intent_errors() {
    let reg = IntentRegistry::new();
    let i = Intent::new("not_registered", json!({}));
    let err = reg.dispatch(&i).unwrap_err();
    assert_eq!(
        err,
        IntentError::UnknownIntent("not_registered".to_string())
    );
}

#[test]
fn registry_register_replaces_existing() {
    let reg = IntentRegistry::new().register(CancelHandler);
    struct CancelHandler2;
    impl IntentHandler for CancelHandler2 {
        fn name(&self) -> &'static str {
            "cancel"
        }
        fn handle(&self, _payload: &Value) -> Result<(), IntentError> {
            Err(IntentError::TypeMismatch {
                key: "replaced",
                expected: "ok",
            })
        }
    }
    let reg2 = reg.register(CancelHandler2);
    let i = Intent::unit("cancel");
    let err = reg2.dispatch(&i).unwrap_err();
    assert!(matches!(err, IntentError::TypeMismatch { .. }));
}

#[test]
fn registry_names_iterator() {
    let reg = IntentRegistry::new()
        .register(MoveHandler)
        .register(CancelHandler);
    let names: Vec<_> = reg.names().collect();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"move"));
    assert!(names.contains(&"cancel"));
}
