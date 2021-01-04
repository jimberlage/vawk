use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub struct ByteTrie {
    children: HashMap<u8, ByteTrie>,
}

pub enum Membership {
    NotIncluded,
    Included,
    IncludedAndTerminal,
}

impl ByteTrie {
    pub fn new() -> ByteTrie {
        ByteTrie {
            children: HashMap::new(),
        }
    }

    pub fn insert(&mut self, path: &[u8]) {
        if path.is_empty() {
            return;
        }

        let head = path.first().unwrap();
        match self.children.get_mut(head) {
            Some(child) => child.insert(&path[1..]),
            None => {
                self.children.insert(*head, ByteTrie::new());
            }
        }
    }

    pub fn membership(&self, path: &[u8]) -> Membership {
        if path.is_empty() {
            return Membership::NotIncluded;
        }

        let head = path.first().unwrap();
        match self.children.get(head) {
            Some(child) if path.len() == 1 && child.is_empty() => Membership::IncludedAndTerminal,
            Some(_) if path.len() == 1 => Membership::Included,
            Some(child) => child.membership(&path[1..]),
            None => Membership::NotIncluded,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    pub fn merge(&mut self, other: ByteTrie) {
        if other.is_empty() {
            return;
        }

        for (byte, other_child) in other.children.into_iter() {
            match self.children.get_mut(&byte) {
                Some(child) => child.merge(other_child),
                None => {
                    self.children.insert(byte, other_child);
                }
            }
        }
    }
}

#[cfg(test)]
mod test {}
