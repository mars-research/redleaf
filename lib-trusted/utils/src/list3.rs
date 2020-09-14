
// Based on https://rust-unofficial.github.io/too-many-lists/fourth-final.html
// list2 but not synced
// A doubly-linked-list with the ability to remove a node from the list in O(1).
use alloc::rc::Rc;
use core::cell::RefCell;
use core::ops::{Deref, DerefMut};


pub struct List<T> {
    pub head: Link<T>,
    pub tail: Link<T>,
}

pub type Pointer<T> = Rc<RefCell<Node<T>>>;
pub type Link<T> = Option<Pointer<T>>;

pub struct Node<T> {
    pub elem: T,
    pub next: Link<T>,
    pub prev: Link<T>,
}


impl<T> Node<T> {
    fn new(elem: T) -> Pointer<T> {
        Rc::new(RefCell::new(Node {
            elem: elem,
            prev: None,
            next: None,
        }))
    }
}

impl<T> Deref for Node<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.elem
    }
}

impl<T> DerefMut for Node<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.elem
    }
}

impl<T> List<T> {
    pub fn new() -> Self {
        List { head: None, tail: None }
    }

    // Allocate a new node and push it to the front
    pub fn push_front(&mut self, elem: T) {
        self.push_node_front(Node::new(elem));
    }

    // Push an existing node to the front
    fn push_node_front(&mut self, new_head: Pointer<T>) {
        match self.head.take() {
            Some(old_head) => {
                old_head.borrow_mut().prev = Some(new_head.clone());
                new_head.borrow_mut().next = Some(old_head);
                self.head = Some(new_head);
            }
            None => {
                self.tail = Some(new_head.clone());
                self.head = Some(new_head);
            }
        }
    }

    pub fn push_back(&mut self, elem: T) {
        let new_tail = Node::new(elem);
        match self.tail.take() {
            Some(old_tail) => {
                old_tail.borrow_mut().next = Some(new_tail.clone());
                new_tail.borrow_mut().prev = Some(old_tail);
                self.tail = Some(new_tail);
            }
            None => {
                self.head = Some(new_tail.clone());
                self.tail = Some(new_tail);
            }
        }
    }

    pub fn pop_back(&mut self) -> Option<T> {
        self.tail.take().map(|old_tail| {
            match old_tail.borrow_mut().prev.take() {
                Some(new_tail) => {
                    new_tail.borrow_mut().next.take();
                    self.tail = Some(new_tail);
                }
                None => {
                    self.head.take();
                }
            }
            Rc::try_unwrap(old_tail).ok().unwrap().into_inner().elem
        })
    }

    pub fn pop_front(&mut self) -> Option<T> {
        self.head.take().map(|old_head| {
            match old_head.borrow_mut().next.take() {
                Some(new_head) => {
                    new_head.borrow_mut().prev.take();
                    self.head = Some(new_head);
                }
                None => {
                    self.tail.take();
                }
            }
            Rc::try_unwrap(old_head).ok().unwrap().into_inner().elem
        })
    }

    // Helper method for move_front.
    fn pop_node(&mut self, node: &mut Node<T>) {
        let prev = node.prev.take();
        let next = node.next.take();

        match &prev {
            Some(prev) => prev.borrow_mut().next = next.clone(),
            None => {
                core::mem::replace(&mut self.head, next.clone());
            },
        }

        match &next {
            Some(next) => next.borrow_mut().prev = prev.clone(),
            None => {
                core::mem::replace(&mut self.tail, prev.clone());
            },
        }
    }

    // Move an existing node to the front
    // Behavior is undefined if the node is not in the list
    pub fn move_front(&mut self, node: Pointer<T>) {
        self.pop_node(&mut node.borrow_mut());
        self.push_node_front(node);
    }

    pub fn iter(&self) -> Iter<T> {
        Iter{ curr: self.head.clone() }
    }

    pub fn rev(&self) -> RevIter<T> {
        RevIter{ curr: self.tail.clone() }
    }
}

impl<T> Drop for List<T> {
    fn drop(&mut self) {
        while self.pop_front().is_some() {}
    }
}

// Well this is ugly since it exposes the internal implementation, but we can't figure out how to do it nicely.
pub struct Iter<T> {
    curr: Link<T>
}

impl<T> Iterator for Iter<T> {
    type Item = Pointer<T>;
    fn next(&mut self) -> Option<Self::Item> {
        self.curr.take().map(|node| {
            self.curr = node.borrow().next.clone();
            node.clone()
        })
    }
}

pub struct RevIter<T> {
    curr: Link<T>
}

impl<T> Iterator for RevIter<T> {
    type Item = Pointer<T>;
    fn next(&mut self) -> Option<Self::Item> {
        self.curr.take().map(|node| {
            self.curr = node.borrow().prev.clone();
            node.clone()
        })
    }
}

