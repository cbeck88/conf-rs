//! Helper for traversing a tree of data (program options) defined in static buffers

#[doc(hidden)]
#[derive(Copy, Debug)]
pub struct LazyBuf<T: 'static> {
    pub buffer: &'static [Node<T>],
    pub transform: fn(&T) -> T,
}

impl<T: 'static> Clone for LazyBuf<T> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            transform: self.transform,
        }
    }
}

impl<T> LazyBuf<T> {
    pub fn iter(&self) -> impl ExactSizeIterator<Item = T> {
        LazyBufIter::<T>::from(self.clone())
    }
    pub fn count(&self) -> usize {
        count(self.buffer)
    }
}

#[doc(hidden)]
#[derive(Debug)]
pub enum Node<T: 'static> {
    Leaf(T),
    Branch(LazyBuf<T>),
}

impl<T> Node<T> {
    pub fn count(&self) -> usize {
        match self {
            Node::Leaf(_) => 1,
            Node::Branch(b) => b.count(),
        }
    }
}

impl<T> From<T> for Node<T> {
    fn from(src: T) -> Self {
        Self::Leaf(src)
    }
}

struct LazyBufIter<T: 'static> {
    stack: Vec<(LazyBuf<T>, usize)>,
}

impl<T: 'static> From<LazyBuf<T>> for LazyBufIter<T> {
    fn from(src: LazyBuf<T>) -> Self {
        Self {
            stack: vec![(src, 0)],
        }
    }
}

impl<T: 'static> Iterator for LazyBufIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        while let Some(&mut (ref lb, ref mut idx)) = self.stack.last_mut() {
            let Some(node) = lb.buffer.get(*idx) else {
                self.stack.pop();
                continue;
            };
            *idx += 1;

            match node {
                Node::Leaf(t) => {
                    let mut result = (lb.transform)(t);
                    for (lb, _) in self.stack.iter().rev().skip(1) {
                        result = (lb.transform)(&result);
                    }
                    return Some(result);
                }
                Node::Branch(b) => {
                    self.stack.push((b.clone(), 0));
                }
            }
        }
        None
    }
}

impl<T: 'static> ExactSizeIterator for LazyBufIter<T> {
    fn len(&self) -> usize {
        self.stack
            .iter()
            .map(|(lb, idx)| count(&lb.buffer[*idx..]))
            .sum()
    }
}

// Helper for summing the count of a set of nodes
fn count<T>(nodes: &[Node<T>]) -> usize {
    nodes.iter().map(Node::count).sum()
}
