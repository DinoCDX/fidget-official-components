#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Viewport {
    pub start: usize,
    pub selected: usize,
    pub total: usize,
    pub capacity: usize,
}

impl Viewport {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            ..Self::default()
        }
    }

    pub fn reset(&mut self) {
        self.start = 0;
        self.selected = 0;
        self.total = 0;
    }

    pub fn set_capacity(&mut self, capacity: usize) {
        self.capacity = capacity.max(1);
        self.start = self.start.min(self.max_start());
        self.follow_selection();
    }

    pub fn set_total(&mut self, total: usize) {
        self.total = total;
        self.selected = self.selected.min(total.saturating_sub(1));
        self.start = self.start.min(self.max_start());
        self.follow_selection();
    }

    pub fn visible(&self) -> std::ops::Range<usize> {
        self.start..(self.start + self.capacity).min(self.total)
    }

    pub fn step(&mut self, delta: isize) {
        if self.total == 0 {
            return;
        }
        self.selected = self
            .selected
            .saturating_add_signed(delta)
            .min(self.total - 1);
        self.follow_selection();
    }

    pub fn page(&mut self, direction: isize) {
        if self.total == 0 {
            return;
        }
        let jump = self.capacity as isize * direction;
        self.start = self.start.saturating_add_signed(jump).min(self.max_start());
        self.selected = self.start;
    }

    fn max_start(&self) -> usize {
        self.total.saturating_sub(self.capacity)
    }

    fn follow_selection(&mut self) {
        if self.selected < self.start {
            self.start = self.selected;
        } else if self.selected >= self.start + self.capacity {
            self.start = self.selected + 1 - self.capacity;
        }
        self.start = self.start.min(self.max_start());
    }
}

#[cfg(test)]
mod tests {
    use super::Viewport;

    #[test]
    fn arrows_scroll_one_row_at_the_edge() {
        let mut viewport = Viewport::new(3);
        viewport.set_total(5);
        viewport.step(1);
        viewport.step(1);
        viewport.step(1);
        assert_eq!((viewport.start, viewport.selected), (1, 3));
    }

    #[test]
    fn tab_pages_forward_by_capacity() {
        let mut viewport = Viewport::new(3);
        viewport.set_total(5);
        viewport.page(1);
        assert_eq!((viewport.start, viewport.selected), (2, 2));
    }

    #[test]
    fn shift_tab_returns_to_the_previous_page() {
        let mut viewport = Viewport::new(2);
        viewport.set_total(6);
        viewport.page(1);
        viewport.page(-1);
        assert_eq!((viewport.start, viewport.selected), (0, 0));
    }

    #[test]
    fn visible_never_exceeds_capacity() {
        let mut viewport = Viewport::new(3);
        viewport.set_total(10);
        for _ in 0..20 {
            viewport.step(1);
            assert!(viewport.visible().len() <= 3);
        }
    }
}
