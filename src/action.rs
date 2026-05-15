/// Decoded user intent. Keys map to `Action`s in `keymap`; the dispatcher in
/// `app` is the only place that knows how each action mutates state.
#[derive(Clone, Copy, Debug)]
pub enum Action {
    Quit,
    MoveSelection(i32),
    SwitchPane,
    SwitchPaneBack,
    Refresh,
    StageSelected,
    UnstageSelected,
    DiscardSelected,
    Commit,
    Dismiss,
}
