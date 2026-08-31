use super::*;
use crate::app::sniff::ProjectRow;
use std::path::PathBuf;

fn app_with_project() -> App {
    let mut app = App::new();
    app.den_dir = PathBuf::from("/tmp/den");
    app.sniff_state.projects = vec![ProjectRow {
        name: "a".into(),
        language: None,
        frameworks: vec![],
        size_bytes: 0,
        is_git_repo: false,
        path: PathBuf::from("/tmp/a"),
    }];
    app.sniff_state.table_state.select(Some(0));
    app
}

fn project_path() -> PathBuf {
    PathBuf::from("/tmp/a")
}

#[test]
fn open_operation_routes_raid_to_raid_flow() {
    let (tx, rx) = mpsc::channel::<WorkerMsg>();
    let mut app = app_with_project();
    app.operations_state.selected = OperationKind::Raid;

    handle_app_command(Command::OpenOperation, &tx, &mut app);

    let flow = app
        .raid_flow
        .as_ref()
        .expect("activating Raid must open the raid flow");
    assert_eq!(flow.project, project_path());
    match rx.recv_timeout(Duration::from_secs(1)) {
        Ok(WorkerMsg::RaidPreview { project, .. }) => {
            assert_eq!(project, project_path());
        }
        other => panic!("expected RaidPreview, got {other:?}"),
    }
}

#[test]
fn open_operation_pack_stash_rinse_route_to_stub_only() {
    for kind in [
        OperationKind::Pack,
        OperationKind::Stash,
        OperationKind::Rinse,
    ] {
        let (tx, rx) = mpsc::channel::<WorkerMsg>();
        let mut app = app_with_project();
        app.operations_state.selected = kind;

        handle_app_command(Command::OpenOperation, &tx, &mut app);

        assert!(
            app.raid_flow.is_none(),
            "{kind:?} must not open a raid flow"
        );
        assert_eq!(app.operations_state.stub, Some(kind));
        assert!(
            rx.try_recv().is_err(),
            "{kind:?} stub must not dispatch work to the worker"
        );
    }
}

#[test]
fn open_operation_without_project_is_a_noop() {
    let (tx, rx) = mpsc::channel::<WorkerMsg>();
    let mut app = App::new();
    app.den_dir = PathBuf::from("/tmp/den");
    app.operations_state.selected = OperationKind::Raid;

    handle_app_command(Command::OpenOperation, &tx, &mut app);

    assert!(app.raid_flow.is_none(), "no project → no raid flow");
    assert!(app.operations_state.stub.is_none());
    assert!(
        rx.try_recv().is_err(),
        "nothing must be sent to the worker without a project"
    );
}

#[test]
fn open_operation_keeps_selection_after_activation() {
    let (tx, _rx) = mpsc::channel::<WorkerMsg>();
    let mut app = app_with_project();
    app.operations_state.selected = OperationKind::Stash;

    handle_app_command(Command::OpenOperation, &tx, &mut app);

    assert_eq!(app.operations_state.selected, OperationKind::Stash);
}
