use avt::terminal::Cursor;
use avt::{Line, Vt};

/// Owned snapshot of terminal state at one point in time.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub lines: Vec<Line>,
    pub cursor: Cursor,
}

/// Thin boundary around `avt`: feed output, apply resizes, take snapshots.
/// All ANSI/DEC semantics live in `avt`; this layer owns nothing terminal-like.
pub struct Term {
    vt: Vt,
}

impl Term {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            vt: Vt::new(cols, rows),
        }
    }

    pub fn feed(&mut self, data: &str) {
        self.vt.feed_str(data);
    }

    pub fn resize(&mut self, cols: usize, rows: usize) {
        self.vt.resize(cols, rows);
    }

    pub fn size(&self) -> (usize, usize) {
        self.vt.size()
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            lines: self.vt.view().cloned().collect(),
            cursor: self.vt.cursor(),
        }
    }
}
