//! # Lazily populated [`nodes`]
//!
//! ![](https://github.com/mnmun/images/blob/main/cherry.png?raw=true)
//!
//! Provides the foundation of the lazily populated [`nodes`]:
//!
//! - [`Node`] - a lazily populated [`tree`] [`node`];
//! - [`Callback`] - a function used to create child [`nodes`] on demand;
//! - [`Link`] - a [`non-null`] pointer to a [`node`].
//!
//! ---
//!
//! See the [`crate documentation`] for more information.
//!
//! [`node`]: Node
//! [`nodes`]: Node
//! [`tree`]: crate::Tree
//! [`non-null`]: NonNull
//! [`crate documentation`]: crate
use std::{cell::UnsafeCell, fmt::Debug, ptr::NonNull, sync::Mutex};

/// # [`NonNull`] pointer to a [`node`]
///
/// ![](https://github.com/mnmun/images/blob/main/chain.png?raw=true)
///
/// `Link` is used to connect a [`node`] with its parent and children.
///
/// ---
///
/// See the [`module documentation`] for more information.
///
/// [`node`]: Node
/// [`module documentation`]: crate::node
pub type Link<Value, Error> = NonNull<Node<Value, Error>>;

/// # Function used to create child [`nodes`]
///
/// ![](https://github.com/mnmun/images/blob/main/telephone.png?raw=true)
///
/// Invoked when a [`cursor`] enters a [`node`] that is not currently being
/// visited by any [`cursor`].
///
/// The `callback` accepts a shared reference to the parent's value.
///
/// On success, it returns a boxed slice containing [`links`] to the created
/// children. Otherwise, it returns an `error`.
///
/// ## Example
///
/// ![](https://github.com/mnmun/images/blob/main/bulb.png?raw=true)
///
/// The following example shows a simple `populate` function that can be used
/// as a `callback` for a [`node`]. The function creates two children and
/// assigns their values as "left" and "right":
///
/// ```rust
/// use std::{ops::Range, borrow::Cow};
///
/// use lazy_tree::node::{Link, Node};
///
/// // A type alias for a data, stored in the nodes
/// type MyValue<'a> = &'a str;
///
/// enum MyError {
///     PopulationFailed,
/// };
///
/// // A type alias for `Link` with the type parameters specified
/// type MyLink<'a> = Link<MyValue<'a>, MyError>;
///
/// // A type alias for a boxed slice of `MyLink`
/// type Children<'a> = Box<[MyLink<'a>]>;
///
/// fn populate<'a>(
///     parent: &MyValue,
/// ) -> Result<Children<'a>, MyError> {
///
///     // Callback can return user-defined error if population may fail
///     // if some_condition {
///     //     return Err(MyError::PopulationFailed)
///     // }
///
///     let left_child = Node::new("left", populate);
///     let right_child = Node::new("right", populate);
///
///     Ok(Box::new([left_child, right_child]))
/// }
/// ```
///
/// ---
///
/// See the [`module documentation`] for more information.
///
/// [`node`]: Node
/// [`node's`]: Node
/// [`nodes`]: Node
/// [`cursor`]: crate::Cursor
/// [`cursors`]: crate::Cursor
/// [`link`]: Link
/// [`links`]: Link
/// [`module documentation`]: crate::node
pub type Callback<Value, Error> =
    fn(&Value) -> Result<Box<[Link<Value, Error>]>, Error>;

/// # Deallocates the subtree rooted at the [`link`]
///
/// [`link`]: Link
pub(crate) fn drop_link<Value, Error>(link: Link<Value, Error>) {
    let mut delete_queue = vec![link];
    while let Some(mut node) = delete_queue.pop() {
        let node = unsafe { Box::from_raw(node.as_mut()) };
        let children = unsafe { node.children.get().replace(Box::default()) };
        delete_queue.extend(children);
    }
}

/// # Lazily populated [`tree`] `node`
///
/// ![](https://github.com/mnmun/images/blob/main/cherry.png?raw=true)
///
/// A `node` stores a used-defined [`value`] as well as a [`callback`] to lazily
/// create its child `nodes`. `Nodes` are connected through [`links`] -
/// [`non-null`] pointers to heap-allocated `nodes`. [`Links`] are used both
/// for the optional parent and for currently populated children.
///
/// ## Creation
///
/// `Node` can be obtained using the [`Node::new()`] method only in the form of
/// a [`link`].
///
/// ## Children
///
/// `Node` has an internal counter that tracks how many [`cursors`] are
/// currently visiting it. When a [`cursor`] descends to (visits) a `node`,
/// its counter is incremented; when a [`cursor`] ascends from (leaves) the
/// `node` - decremented.
///
/// When a [`cursor`] visits a `node` whose current counter is zero, the
/// `node's` [`callback`] is invoked. The resulting children remain available
/// until the last [`cursor`] leaves the `node`, at which point the counter
/// reaches zero and children are released.
///
/// ---
///
/// See the [`module documentation`] for more information.
///
/// [`tree`]: crate::Tree
/// [`non-null`]: NonNull
/// [`value`]: Node::value()
/// [`link`]: Link
/// [`links`]: Link
/// [`Links`]: Link
/// [`cursors`]: crate::Cursor
/// [`cursor`]: crate::Cursor
/// [`callback`]: Callback
/// [`module documentation`]: crate::node
pub struct Node<Value, Error> {
    pub(crate) parent: Option<Link<Value, Error>>,
    pub(crate) children: UnsafeCell<Box<[Link<Value, Error>]>>,
    pub(crate) visitors: Mutex<usize>,

    value: Value,
    callback: Callback<Value, Error>,
}

impl<Value, Error> Debug for Node<Value, Error> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("parent", &self.parent)
            .field("children", unsafe { &*self.children.get() })
            .field("visitors", &self.visitors)
            .finish()
    }
}

impl<Value, Error> Node<Value, Error> {
    /// # Allocates a [`node`] and returns a [`link`] to it
    ///
    /// [`node`]: Node
    /// [`link`]: Link
    pub fn new(
        value: impl Into<Value>,
        callback: Callback<Value, Error>,
    ) -> Link<Value, Error> {
        let value = value.into();

        let node = Node {
            parent: None,
            children: UnsafeCell::default(),
            visitors: Mutex::new(0),

            value,
            callback,
        };

        #[cfg(feature = "debug")]
        {
            use crate::tree::NODES_ALIVE;
            use std::sync::atomic::Ordering;
            NODES_ALIVE.fetch_add(1, Ordering::Relaxed);
        }

        unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(node))) }
    }

    /// # Returns a reference to the [`node's`] `value`
    ///
    /// [`node's`]: Node
    pub fn value(&self) -> &Value {
        &self.value
    }

    /// # Returns the shared references to the currently populated child [`nodes`]
    ///
    /// [`nodes`]: Node
    pub fn children(&self) -> Box<[&Node<Value, Error>]> {
        unsafe {
            (*self.children.get())
                .iter()
                .map(|child| child.as_ref())
                .collect()
        }
    }

    /// # Releases all children of the [`node`]
    ///
    /// [`node`]: Node
    fn kill_children(&self) {
        for child in unsafe { self.children.get().replace(Box::default()) } {
            drop_link(child);
        }
    }

    /// # Populates the [`node's`] child [`nodes`] using [`callback`]
    ///
    /// Any previously populated children are released.
    ///
    /// [`node's`]: Node
    /// [`nodes`]: Node
    /// [`callback`]: Callback
    fn make_children(
        &self,
        link_to_itself: Link<Value, Error>,
    ) -> Result<(), Error> {
        let mut children = (self.callback)(&self.value)?;

        children.iter_mut().for_each(|child| unsafe {
            child.as_mut().parent = Some(link_to_itself)
        });

        for child in unsafe { self.children.get().replace(children) } {
            drop_link(child);
        }

        Ok(())
    }

    /// # Convenient wrapper for an `unsafe { link.as_ref() }` call
    pub(crate) fn from_link<'a>(link: Link<Value, Error>) -> &'a Self {
        unsafe { link.as_ref() }
    }

    /// # Register [`cursor`] as a visitor of this [`node`]
    ///
    /// When a [`cursor`] descends from a [`node`] `A` to a [`node`] `B`, this
    /// method is invoked for a [`node`] `B`. This occurs during [`walk()`] and
    /// [`jump()`] methods with [`Direction::Down`].
    ///
    /// Keep in mind that horizontal movement using [`walk()`] and [`jump()`]
    /// methods is treated as a sequence of upward and downward movements.
    ///
    /// This method increments the `visitors` counter and invokes the [`node's`]
    /// [`callback`] if previous value of `visitors` was zero.
    ///
    /// # Errors
    ///
    /// If the [`callback`] returns an `error`, the `visitors` counter is not
    /// incremented, and the children are populated on the next successful
    /// attempt to visit this [`node`].
    ///
    /// [`cursor`]: crate::Cursor
    /// [`walk()`]: crate::Cursor::walk()
    /// [`jump()`]: crate::Cursor::jump()
    /// [`Direction::Down`]: crate::Direction::Down
    /// [`node`]: Node
    /// [`node's`]: Node
    /// [`callback`]: Callback
    pub(crate) fn visit(
        &self,
        link_to_itself: Link<Value, Error>,
    ) -> Result<(), Error> {
        let mut visitors = match self.visitors.lock() {
            Ok(guard) => guard,
            Err(poison) => poison.into_inner(),
        };

        if *visitors == 0 {
            self.make_children(link_to_itself)?;
        }

        *visitors += 1;

        Ok(())
    }

    /// # Unregisters [`cursor`] as a visitor of this [`node`]
    ///
    /// When a [`cursor`] ascends from a [`node`] `B` to a [`node`] `A`, this
    /// method is invoked for a [`node`] `B`. This occurs during [`walk()`] and
    /// [`jump()`] methods with [`Direction::Up`].
    ///
    /// Keep in mind that horizontal movement using [`walk()`] and [`jump()`]
    /// methods is treated as a sequece of upward and downward movements.
    ///
    /// This method decrements the `visitors` counter and releases the
    /// [`node's`] children when the new value of `visitors` is zero.
    ///
    /// [`cursor`]: crate::Cursor
    /// [`walk()`]: crate::Cursor::walk()
    /// [`jump()`]: crate::Cursor::jump()
    /// [`Direction::Up`]: crate::Direction::Up
    /// [`node`]: Node
    /// [`node's`]: Node
    pub(crate) fn leave(&self) {
        let mut visitors = match self.visitors.lock() {
            Ok(guard) => guard,
            Err(poison) => poison.into_inner(),
        };

        if *visitors == 1 {
            self.kill_children();
        }

        *visitors = visitors
            .checked_sub(1)
            .expect("Visitors count could not be zero at this point");
    }
}

impl<Value, Error> Drop for Node<Value, Error> {
    fn drop(&mut self) {
        #[cfg(feature = "debug")]
        {
            use crate::tree::NODES_ALIVE;
            use std::sync::atomic::Ordering;
            NODES_ALIVE.fetch_sub(1, Ordering::Relaxed);
        }
    }
}
