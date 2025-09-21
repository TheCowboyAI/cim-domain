use cim_domain::{
    DomainEvent, MealyStateTransitions, TransactionInput as I, TransactionState as S,
};

const FEATURE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../doc/qa/features/transaction_state.feature"
));

#[derive(Debug, Clone)]
struct World {
    state: S,
    events: Vec<String>,
    last_transition_valid: Option<bool>,
}
impl Default for World {
    fn default() -> Self {
        Self {
            state: S::Idle,
            events: vec![],
            last_transition_valid: None,
        }
    }
}

fn apply(world: &mut World, input: I) {
    let targets = world.state.valid_transitions(&input);
    if let Some(next) = targets.first().cloned() {
        let out = world.state.transition_output(&next, &input);
        for event in out.events.iter() {
            world
                .events
                .push(DomainEvent::event_type(event.as_ref()).to_string());
        }
        world.state = next;
        world.last_transition_valid = Some(true);
    } else {
        world
            .events
            .push("TransactionTransitionRejected".to_string());
        world.last_transition_valid = Some(false);
    }
}

#[test]
fn bdd_transaction_state_feature() {
    for block in FEATURE.split("\n\n").map(str::trim) {
        if block.starts_with("Scenario:") {
            let mut world = World::default();
            for line in block.lines().map(str::trim) {
                if line.starts_with("Given Transaction is Idle") {
                    world.state = S::Idle;
                } else if line.starts_with("When we Start") {
                    apply(&mut world, I::Start);
                } else if line.starts_with("And we ValidateOk") {
                    apply(&mut world, I::ValidateOk);
                } else if line.starts_with("And we ValidateFail") {
                    apply(&mut world, I::ValidateFail);
                } else if line.starts_with("And we Commit") || line.starts_with("When we Commit") {
                    apply(&mut world, I::Commit);
                } else if line.starts_with("And we Cancel") || line.starts_with("When we Cancel") {
                    apply(&mut world, I::Cancel);
                } else if line.starts_with("Then state is Committed") {
                    assert_eq!(world.state, S::Committed);
                } else if line.starts_with("Then state is Cancelled") {
                    assert_eq!(world.state, S::Cancelled);
                } else if line.starts_with("Then state is Failed") {
                    assert_eq!(world.state, S::Failed);
                } else if let Some(rest) = line.strip_prefix("And Expect Event Stream is ") {
                    let rest = rest.trim();
                    if rest.eq_ignore_ascii_case("empty") {
                        assert!(
                            world.events.is_empty(),
                            "expected empty stream, got {:?}",
                            world.events
                        );
                    } else {
                        let expected = parse_event_list(rest);
                        assert_eq!(world.events, expected, "event stream mismatch");
                    }
                } else if line.starts_with("Then transition is invalid") {
                    assert_eq!(world.last_transition_valid, Some(false));
                }
            }
        }
    }
}

fn parse_event_list(expr: &str) -> Vec<String> {
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let inner = trimmed
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .expect("event list must be wrapped in [ ]");
    if inner.trim().is_empty() {
        return Vec::new();
    }
    inner
        .split(',')
        .map(|item| item.trim().trim_matches('"').to_string())
        .collect()
}
