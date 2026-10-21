use crate::Grade;

/// What the menu-bar dot shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Measuring,
    Graded(Grade),
    Offline,
}
