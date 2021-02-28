//! A truly terrible linked list implementation that flattens all its nodes onto
//! a Vec for storage, gaining data locality and O(1) random access at the
//! expense of any semblance of resizeability.

/// Alias for the index of a node in the linked list's storage vec.
#[derive(Copy, Clone)]
pub(crate) struct NodeHandle(usize);

/// A node that lives in a linked list.
pub(crate) struct Node<T>
where
    T: Eq + Copy,
{
    /// The value being stored.
    value: T,

    /// The index of the node previous to this one.
    prev: NodeHandle,

    /// The index of the next node in the list.
    next: NodeHandle,
}

/// This is the funniest linked list y'all ever did see. It has a single
/// continguous Vec of nodes, which are addressed by their position in the Vec.
/// It freelists elements that have been removed. It has no compaction, it's
/// designed for use in situations where it will be at a determined size for its
/// whole life, and is optimized for random-access node removal.
///
/// Why? By doing this we can keep the nodes closer together and make it easier
/// for the CPU's cache system to reason about them. It's trying to get the best
/// of O(1) insertion/removal of a LinkedList with the low-level memory handling
/// of an array.
///
/// Leaking a NodeHandle outside the linked list is important for LruCache,
/// which can at any time pull a node out of the eviction queue and put it back
/// in at the front (when something is most recent as opposed to least-recent).
pub(crate) struct LinkedList<T>
where
    T: Eq + Copy,
{
    /// The nodes in the list.
    pub(crate) store: Vec<Node<T>>,

    /// Entries in the list which aren't in use anymore. These will be reused.
    free: Vec<NodeHandle>,

    /// The first node in the list.
    head: Option<NodeHandle>,
}

impl<T> LinkedList<T>
where
    T: Eq + Copy,
{
    /// Creates a new linked list with a specific capacity.
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            store: Vec::with_capacity(capacity),
            free: Vec::with_capacity(capacity),
            head: None,
        }
    }

    /// The length of this linked list.
    pub(crate) fn len(&self) -> usize {
        self.store.len() - self.free.len()
    }

    /// Pushes t onto the front of the list and returns a handle to the node.
    pub(crate) fn push(&mut self, t: T) -> NodeHandle {
        let mut n = Node {
            value: t,
            prev: NodeHandle(0),
            next: NodeHandle(0),
        };

        // use the first available location in the storage vec, or infer what
        // the next location will be on push.
        let idx = self
            .free
            .pop()
            .unwrap_or_else(|| NodeHandle(self.store.len()));

        if let Some(head) = self.head {
            // link this node into the chain
            n.prev = self.store[head.0].prev;
            n.next = head;

            self.store[n.prev.0].next = idx;
            self.store[n.next.0].prev = idx;
        }

        self.head = Some(idx);

        if self.store.len() <= idx.0 {
            self.store.push(n);
        } else {
            self.store[idx.0] = n;
        }

        return idx;
    }

    /// Pops the back node off the list if it exists.
    pub(crate) fn pop_back(&mut self) -> Option<T> {
        if let Some(head) = self.head {
            let head_prev = self.store[head.0].prev;
            let prev = self.store[head_prev.0].value;
            self.remove_node(&head_prev);
            return Some(prev);
        }

        return None;
    }

    /// Remove an arbitrary node from the list.
    pub(crate) fn remove_node(&mut self, node: &NodeHandle) {
        if self.len() == 1 {
            // just reset head and freelist the node
            self.head = None;
        } else {
            if node.0 == self.head.unwrap().0 {
                self.head = Some(self.store[node.0].next);
            }

            // link prev to next and next to prev so node doesn't exist in the
            // chain anymore; it'll get overwritten at some later push by
            // placing its handle on the freelist
            let prev = self.store[node.0].prev;
            let next = self.store[node.0].next;

            self.store[prev.0].next = self.store[node.0].next;
            self.store[next.0].prev = self.store[node.0].prev;
        }

        self.free.push(NodeHandle(node.0));
    }

    /// Clears this linked list. Does not free the underlying buffers.
    pub(crate) fn clear(&mut self) {
        self.store.clear();
        self.free.clear();
        self.head = None;
    }
}
