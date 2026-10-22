//! What the menu-bar dot shows: the verdict, in the colours of the system's scheme. The verdict
//! comes with each display, the scheme from a watcher that reads it each second: either change
//! alone redraws the dot, even while no display comes (a throughput test).

use std::sync::{Mutex, PoisonError};

use application::ColorSchemeReader;
use domain::{ColorScheme, Verdict};

/// Where the dot is drawn: the tray icon, or a fake in tests.
pub trait TrayCanvas {
    /// `false` when the drawing failed: the dot is drawn again at the next change.
    fn paint(&self, verdict: Verdict, scheme: ColorScheme, label: &str) -> bool;
}

struct Shown {
    verdict: Verdict,
    scheme: ColorScheme,
    label: String,
}

pub struct TrayPainter<C, R> {
    canvas: C,
    reader: R,
    shown: Mutex<Shown>,
}

impl<C: TrayCanvas, R: ColorSchemeReader> TrayPainter<C, R> {
    /// `canvas` must already show `verdict` in the colours of `scheme`, labelled `label`.
    pub fn new(canvas: C, reader: R, verdict: Verdict, scheme: ColorScheme, label: String) -> Self {
        Self {
            canvas,
            reader,
            shown: Mutex::new(Shown {
                verdict,
                scheme,
                label,
            }),
        }
    }

    /// O(1) when nothing changed.
    pub fn show_verdict(&self, verdict: Verdict, label: &str) {
        let mut shown = self.shown.lock().unwrap_or_else(PoisonError::into_inner);
        if shown.verdict == verdict && shown.label == label {
            return;
        }
        if self.canvas.paint(verdict, shown.scheme, label) {
            shown.verdict = verdict;
            label.clone_into(&mut shown.label);
        }
    }

    /// Reads the system's scheme and redraws the dot when it changed. O(1) when nothing changed.
    pub fn follow_scheme(&self) {
        let scheme = self.reader.read_color_scheme();
        let mut shown = self.shown.lock().unwrap_or_else(PoisonError::into_inner);
        if shown.scheme == scheme {
            return;
        }
        if self.canvas.paint(shown.verdict, scheme, &shown.label) {
            shown.scheme = scheme;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use domain::Grade;

    use super::*;

    /// Keeps every drawing, in order.
    #[derive(Default)]
    struct InMemoryCanvas {
        paintings: RefCell<Vec<(Verdict, ColorScheme)>>,
    }

    impl TrayCanvas for &InMemoryCanvas {
        fn paint(&self, verdict: Verdict, scheme: ColorScheme, _label: &str) -> bool {
            self.paintings.borrow_mut().push((verdict, scheme));
            true
        }
    }

    /// The scheme the system shows, set by the test.
    #[derive(Default)]
    struct InMemoryColorSchemeReader(Cell<ColorScheme>);

    impl ColorSchemeReader for &InMemoryColorSchemeReader {
        fn read_color_scheme(&self) -> ColorScheme {
            self.0.get()
        }
    }

    const SOLID: Verdict = Verdict::Graded(Grade::Solid);

    #[test]
    fn redraws_the_same_verdict_when_the_system_turns_dark() {
        // Arrange
        let canvas = InMemoryCanvas::default();
        let reader = InMemoryColorSchemeReader::default();
        let painter = TrayPainter::new(
            &canvas,
            &reader,
            SOLID,
            ColorScheme::Light,
            "Solide".to_owned(),
        );
        reader.0.set(ColorScheme::Dark);

        // Act
        painter.follow_scheme();
        painter.follow_scheme();

        // Assert
        assert_eq!(*canvas.paintings.borrow(), [(SOLID, ColorScheme::Dark)]);
    }

    #[test]
    fn draws_a_new_verdict_in_the_scheme_already_shown() {
        // Arrange
        let canvas = InMemoryCanvas::default();
        let reader = InMemoryColorSchemeReader::default();
        let painter = TrayPainter::new(
            &canvas,
            &reader,
            Verdict::Measuring,
            ColorScheme::Dark,
            "Mesure en cours".to_owned(),
        );

        // Act
        painter.show_verdict(SOLID, "Solide");
        painter.show_verdict(SOLID, "Solide");

        // Assert
        assert_eq!(*canvas.paintings.borrow(), [(SOLID, ColorScheme::Dark)]);
    }
}
