use core::{marker::PhantomPinned, pin::Pin, ptr::NonNull};

pub struct List<T> {
    head: Option<NonNull<Node<T>>>,
    tail: Option<NonNull<Node<T>>>,
}

pub struct Node<T> {
    next: Option<NonNull<Node<T>>>,
    prev: Option<NonNull<Node<T>>>,
    pub data: T,
    _pin: PhantomPinned,
    inserted: bool,
}

impl<T> List<T> {
    pub const fn new() -> Self {
        List {
            head: None,
            tail: None,
        }
    }

    /// Safety: `node` MUST not be moved or dropped before it is removed from the list. Additionally `node` MUST not already be in a list.
    pub unsafe fn push_front(&mut self, node: Pin<&mut Node<T>>) {
        let node = node.get_unchecked_mut();
        node.next = self.head;
        node.prev = None;
        node.inserted = true;

        if let Some(mut head) = self.head {
            head.as_mut().prev = Some(node.into())
        }

        self.head = Some(node.into());

        if self.tail.is_none() {
            self.tail = Some(node.into());
        }
    }

    pub fn peek_front(&self) -> Option<&Node<T>> {
        unsafe {
            let head = self.head?;
            Some(head.as_ref())
        }
    }
}

unsafe impl<T> Send for List<T> where T: Send {}
unsafe impl<T> Sync for List<T> where T: Sync {}

impl<T: core::fmt::Debug> Node<T> {
    pub fn new(data: T) -> Self {
        Node {
            next: None,
            prev: None,
            data,
            inserted: false,
            _pin: PhantomPinned,
        }
    }

    pub fn is_init(&self) -> bool {
        self.inserted
    }

    pub fn next(&self) -> Option<&Node<T>> {
        let next = self.next?;
        unsafe { Some(next.as_ref()) }
    }

    /// Safety: The caller must guarantee that self is contained in `list`.
    pub unsafe fn remove(self: Pin<&mut Self>, list: &mut List<T>) {
        match self.next {
            Some(mut next) => {
                next.as_mut().prev = self.prev;
            }
            None => {
                list.tail = self.prev;
            }
        }

        match self.prev {
            Some(mut prev) => {
                prev.as_mut().next = self.next;
            }
            None => {
                list.head = self.next;
            }
        }

        let this = self.get_unchecked_mut();
        this.next = None;
        this.prev = None;
        this.inserted = false;
    }
}

unsafe impl<T> Send for Node<T> where T: Send {}
unsafe impl<T> Sync for Node<T> where T: Sync {}

#[test]
fn push_iterate() {
    let mut list = List::new();
    let node_a = core::pin::pin!(Node::new(6i32));
    let node_b = core::pin::pin!(Node::new(12i32));
    let node_c = core::pin::pin!(Node::new(18i32));
    unsafe {
        list.push_front(node_a);
        list.push_front(node_b);
        list.push_front(node_c);
    }

    let node_a_ref = list.peek_front().unwrap();
    assert_eq!(18, node_a_ref.data);
    assert_eq!(12, node_a_ref.next().unwrap().data);
    assert_eq!(6, node_a_ref.next().unwrap().next().unwrap().data);
    assert!(node_a_ref.next().unwrap().next().unwrap().next.is_none());
}

#[test]
fn remove() {
    let mut list = List::new();
    {
        let mut node_a = core::pin::pin!(Node::new(6i32));
        let mut node_b = core::pin::pin!(Node::new(12i32));
        let mut node_c = core::pin::pin!(Node::new(18i32));

        unsafe {
            list.push_front(node_a.as_mut());
            list.push_front(node_b.as_mut());
            list.push_front(node_c.as_mut());

            node_b.remove(&mut list);
            node_a.remove(&mut list);
            node_c.remove(&mut list);
        }
    }

    assert!(list.peek_front().is_none());
}
