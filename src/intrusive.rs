use core::{ptr::NonNull, pin::Pin};

pub struct List<T> {
    head: Option<NonNull<Node<T>>>,
}

pub struct Node<T> {
    next: Option<NonNull<Node<T>>>,
    pub data: T,
}

impl<T> List<T> {
    pub fn new() -> Self {
        List {
            head: None,
        }
    }

    /// Safety: `node` MUST not be moved or dropped before it is removed from the list. Additionally `node` MUST not be added to any other list.
    pub unsafe fn push_front(&mut self, node: Pin<&mut Node<T>>) {
        unsafe {
            let node = node.get_unchecked_mut();
            node.next = self.head;
            self.head = Some(NonNull::from(node))
        }
    }

    pub fn pop_front(&mut self) -> Option<Pin<&mut Node<T>>> {
        unsafe {
            let mut head = self.head?;
            let node = head.as_mut();
            self.head = node.next;
            
            node.next = None;

            Some(Pin::new_unchecked(node))
        }
    }
}

impl<T> Node<T> {
    pub fn new(data: T) -> Self {
        Node { next: None, data }
    }
}

#[test]
fn push_pop() {
    let mut list = List::new();
    let node_a = core::pin::pin!(Node::new(6i32));
    let node_b = core::pin::pin!(Node::new(12i32));
    let node_c = core::pin::pin!(Node::new(18i32));
        
    unsafe {
        list.push_front(node_a);
        list.push_front(node_b);
        list.push_front(node_c);
    }
        
    assert_eq!(18, list.pop_front().unwrap().data);
    assert_eq!(12, list.pop_front().unwrap().data);
    assert_eq!(6, list.pop_front().unwrap().data);
    assert!(list.pop_front().is_none());
}