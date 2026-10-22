use relic_cli::dispatch::{parse_dispatch_intent, DispatchIntent};

#[test]
fn test_cli_dispatch_query_with_avoid_constraint() {
    let q = "¿Se puede ir desde Madrid hasta París sin pasar por Barcelona?";
    let intent = parse_dispatch_intent(q);
    if let DispatchIntent::GraphReachability(spec) = intent {
        assert_eq!(spec.initial.as_deref(), Some("Madrid"));
        assert_eq!(spec.target.as_deref(), Some("París"));
        assert_eq!(spec.avoid.as_deref(), Some("Barcelona"));
    } else {
        panic!("Debe ser clasificado como GraphReachability");
    }
}

#[test]
fn test_cli_dispatch_unconstrained_query() {
    let q = "¿Se puede ir desde Madrid hasta París?";
    let intent = parse_dispatch_intent(q);
    if let DispatchIntent::GraphReachability(spec) = intent {
        assert_eq!(spec.initial.as_deref(), Some("Madrid"));
        assert_eq!(spec.target.as_deref(), Some("París"));
        assert_eq!(spec.avoid, None);
    } else {
        panic!("Debe ser clasificado como GraphReachability");
    }
}
