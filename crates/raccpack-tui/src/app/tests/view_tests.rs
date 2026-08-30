//! ViewId cycle/label/key tests.

use super::super::*;

#[test]
fn view_id_next_cycles() {
    assert_eq!(ViewId::Overview.next(), ViewId::Projects);
    assert_eq!(ViewId::Projects.next(), ViewId::Findings);
    assert_eq!(ViewId::Findings.next(), ViewId::Operations);
    assert_eq!(ViewId::Operations.next(), ViewId::Overview);
}

#[test]
fn view_id_prev_round_trips_next() {
    let views = [
        ViewId::Overview,
        ViewId::Projects,
        ViewId::Findings,
        ViewId::Operations,
    ];
    for &view in &views {
        assert_eq!(view.next().prev(), view, "next().prev() must round-trip");
        assert_eq!(view.prev().next(), view, "prev().next() must round-trip");
    }
}

#[test]
fn view_id_prev_cycles() {
    assert_eq!(ViewId::Overview.prev(), ViewId::Operations);
    assert_eq!(ViewId::Projects.prev(), ViewId::Overview);
    assert_eq!(ViewId::Findings.prev(), ViewId::Projects);
    assert_eq!(ViewId::Operations.prev(), ViewId::Findings);
}

#[test]
fn view_id_labels() {
    assert_eq!(ViewId::Overview.label(), "Overview");
    assert_eq!(ViewId::Projects.label(), "Projects");
    assert_eq!(ViewId::Findings.label(), "Findings");
    assert_eq!(ViewId::Operations.label(), "Operations");
}

#[test]
fn view_id_keys() {
    assert_eq!(ViewId::Overview.key(), '1');
    assert_eq!(ViewId::Projects.key(), '2');
    assert_eq!(ViewId::Findings.key(), '3');
    assert_eq!(ViewId::Operations.key(), '4');
}
