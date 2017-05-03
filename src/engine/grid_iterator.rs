//! Defines an object that iterates over of a universe in some order.

pub trait GridIterator {
    fn visit(&mut self) -> Option<usize>;
}

impl Iterator for GridIterator {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        self.visit()
    }
}

pub trait EntityIterator {
    fn visit(&mut self) -> Option<(usize, usize)>;
}

impl Iterator for EntityIterator {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        self.visit()
    }
}
