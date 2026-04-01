/// What to do after handling one node's execution result inside the loop.
pub(super) enum LoopAction {
    Advance,
    Retry,
    Done,
}
