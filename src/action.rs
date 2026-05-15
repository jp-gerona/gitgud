/// Decoded user intent. Keys map to `Action`s in `keymap`; the dispatcher in
/// `app` is the only place that knows how each action mutates state.
#[derive(Clone, Copy, Debug)]
pub enum Action {
    Quit,
    MoveSelection(i32),
    /// `Tab`/`Shift+Tab`: flip between the Unstaged and Staged file panes.
    /// Inert while the Diff pane is focused.
    SwitchPane,
    SwitchPaneBack,
    /// `l`/`→`: move focus from the file panes into the Diff pane.
    EnterDiff,
    /// `h`/`←`: move focus from the Diff pane back to the file panes.
    LeaveDiff,
    Refresh,
    StageSelected,
    UnstageSelected,
    DiscardSelected,
    Commit,
    Dismiss,
}
