//! # Lazily-created [`node`]
//!
//! ![](https://github.com/mnmun/images/blob/main/cherry.png?raw=true)
//!
//! Provides the foundation of the lazily-created [`nodes`]:
//!
//! - [`Node`] - a structural unit of a [`tree`] that is created on an
//!   as-needed basis using the [`query`] callback;
//! - [`Builder`] - a configurable [`builder`] used to create a new [`node`];
//! - [`Link`] - a [`non-null`] pointer to a [`tree`] [`node`], used to
//!   establish a connection between a single parent [`node`] and multiple
//!   child [`nodes`].
//!
//! ---
//!
//! See the [`crate`] documentation for more information.
//!
//! [`node`]: crate::Node
//! [`nodes`]: crate::Node
//! [`tree`]: crate::Tree
//! [`query`]: crate::callbacks::Query
//! [`builder`]: crate::node::Builder
//! [`non-null`]: NonNull
use std::{
    collections::{HashMap, hash_map::Entry},
    fmt::{self, Debug},
    mem::{self, MaybeUninit},
    pin::Pin,
    ptr::{self, NonNull},
    sync::Mutex,
};

#[cfg(test)]
use std::sync::MutexGuard;

use getset::{Getters, MutGetters};

use crate::callbacks::{Callbacks, Query, Total};

/// # [`NonNull`] pointer to a [`node`]
///
/// ![](https://github.com/mnmun/images/blob/main/chain.png?raw=true)
///
/// Connects a parent [`node`] and its child [`nodes`].
///
/// ---
///
/// See the [`module`] documentation for more information.
///
/// [`node`]: crate::Node
/// [`nodes`]: crate::Node
/// [`module`]: crate::node
pub type Link<Value, Error> = NonNull<Node<Value, Error>>;

/// # Deallocates the subtree rooted at the [`link`]
///
/// See the [`node`] documentation for more information.
///
/// [`link`]: crate::node::Link
/// [`node`]: crate::Node
pub(crate) fn drop_link<Value, Error>(link: Link<Value, Error>) {
    let mut delete_queue = vec![link];
    while let Some(mut node) = delete_queue.pop() {
        let node = unsafe { Box::from_raw(node.as_mut()) };

        let mut children = match node.children.lock() {
            Ok(guard) => guard,
            Err(poison) => poison.into_inner(),
        };

        delete_queue.extend(mem::take(&mut *children).values());
    }
}

/// # [`Node`] builder
///
/// Holds the data used for the [`node`] creation.
///
/// ## Creation
///
/// Can be created using the [`Builder::new()`] method, which accepts arguments:
///
/// - `value` - a `value` stored in the resulting [`node`]. Accessible via a
///   [`cursor`];
/// - `total` - a [callback](crate::callbacks::Total) for the `total` number
///   of child [`nodes`]. Calculates the `total` number of child [`nodes`] for
///   the resulting [`node`];
/// - `query` - a [callback](crate::callbacks::Query) to query a child
///   [`node`] by `index`. Returns a [`builder`] for a new child [`node`],
///   specified by `index`, for the resulting [`node`].
///
/// ## Build
///
/// There is no need to call the `build()` method manually - it is intentionally
/// made private and is executed automatically after the [`query`] callback,
/// with a correct [`link`] to the parent [`node`] passed in.
///
/// ---
///
/// See the [`node`] documentation for more information.
///
/// [`node`]: crate::Node
/// [`nodes`]: crate::Node
/// [`cursor`]: crate::Cursor
/// [`builder`]: crate::node::Builder
/// [`query`]: crate::callbacks::Query
/// [`link`]: crate::node::Link
#[derive(Debug, Getters, MutGetters)]
pub struct Builder<Value, Error> {
    /// # Optional [`link`] to the parent [`node`]
    ///
    /// Methods for setting a [`link`] to the parent [`node`] are intentionally
    /// made private to prevent memory leaks due to incorrect use.
    ///
    /// [`Node`] without a parent is considered `root` and requires manual
    /// removal using [`drop_link()`].
    ///
    /// ---
    ///
    /// See the [`builder`] documentation for more information.
    ///
    /// [`builder`]: crate::node::Builder
    /// [`link`]: crate::node::Link
    /// [`node`]: crate::Node
    #[getset(get = "pub(crate)", get_mut = "pub(crate)")]
    parent: Option<Link<Value, Error>>,

    /// # `Value` stored in the resulting [`node`]
    ///
    /// See the [`builder`] documentation for more information.
    ///
    /// [`builder`]: crate::node::Builder
    /// [`node`]: crate::Node
    #[getset(get = "pub", get_mut = "pub")]
    value: Value,

    /// # [`Node`] [callbacks]
    ///
    /// See the [`builder`] documentation for more information.
    ///
    /// [`builder`]: crate::node::Builder
    /// [`Node`]: crate::Node
    /// [callbacks]: crate::callbacks
    callbacks: Callbacks<Value, Error>,
}

impl<Value, Error> Builder<Value, Error> {
    /// # [`Callback`](crate::callbacks::Total) for `total` number of child [`nodes`]
    ///
    /// Calculates the `total` number of child [`nodes`] for the resulting [`node`]
    ///
    /// ---
    ///
    /// See the [`builder`] documentation for more information.
    ///
    /// [`builder`]: crate::node::Builder
    /// [`node`]: crate::Node
    /// [`nodes`]: crate::Node
    fn total(&self) -> &Total<Value, Error> {
        &self.callbacks.total
    }

    /// # [`Callback`](crate::callbacks::Query) to query a child [`node`] by `index`
    ///
    /// Returns a [`builder`] for a new child [`node`], specified by `index`,
    /// for the resulting [`node`].
    ///
    /// ---
    ///
    /// See the [`builder`] documentation for more information.
    ///
    /// [`builder`]: crate::node::Builder
    /// [`node`]: crate::Node
    fn query(&self) -> &Query<Value, Error> {
        &self.callbacks.query
    }
}

impl<Value, Error> Builder<Value, Error> {
    /// # [`Callback`](crate::callbacks::Total) for `total` number of child [`nodes`]
    ///
    /// Calculates the `total` number of child [`nodes`] for the resulting [`node`]
    ///
    /// ---
    ///
    /// See the [`builder`] documentation for more information.
    ///
    /// [`builder`]: crate::node::Builder
    /// [`node`]: crate::Node
    /// [`nodes`]: crate::Node
    fn total_mut(&mut self) -> &mut Total<Value, Error> {
        &mut self.callbacks.total
    }

    /// # [`Callback`](crate::callbacks::Query) to query a child [`node`] by `index`
    ///
    /// Returns a [`builder`] for a new child [`node`], specified by `index`,
    /// for the resulting [`node`].
    ///
    /// ---
    ///
    /// See the [`builder`] documentation for more information.
    ///
    /// [`builder`]: crate::node::Builder
    /// [`node`]: crate::Node
    fn query_mut(&mut self) -> &mut Query<Value, Error> {
        &mut self.callbacks.query
    }
}

impl<Value, Error> Builder<Value, Error> {
    /// # Optional [`link`] to the parent [`node`]
    ///
    /// Methods for setting a [`link`] to the parent [`node`] are intentionally
    /// made private to prevent memory leaks due to incorrect use.
    ///
    /// [`Node`] without a parent is considered `root` and requires manual
    /// removal using [`drop_link()`].
    ///
    /// ---
    ///
    /// See the [`builder`] documentation for more information
    ///
    /// [`builder`]: crate::node::Builder
    /// [`link`]: crate::node::Link
    /// [`node`]: crate::Node
    #[must_use = "method returns the modified value"]
    pub(crate) fn with_parent(
        mut self,
        value: impl Into<Option<Link<Value, Error>>>,
    ) -> Self {
        self.parent = value.into();
        self
    }
}

impl<Value, Error> Builder<Value, Error> {
    /// # Optional [`link`] to the parent [`node`]
    ///
    /// [`Node`] without a parent is considered `root` and requires manual
    /// removal using [`drop_link()`].
    ///
    /// ---
    ///
    /// See the [`builder`] documentation for more information
    ///
    /// [`link`]: crate::node::Link
    /// [`node`]: crate::Node
    /// [`builder`]: crate::node::Builder
    pub(crate) fn set_parent(
        &mut self,
        value: impl Into<Option<Link<Value, Error>>>,
    ) -> &mut Self {
        self.parent = value.into();
        self
    }
}

impl<Value, Error> Builder<Value, Error> {
    /// # Creates a new [`node`] `builder`
    ///
    /// Arguments:
    ///
    /// - `value` - a `value` stored in the resulting [`node`]. Accessible via the
    ///   [`cursor`];
    /// - `total` - a [callback](crate::callbacks::Total) for the `total` number
    ///   of child [`nodes`]. Calculates the `total` number of child [`nodes`] for
    ///   the resulting [`node`];
    /// - `query` - a [callback](crate::callbacks::Query) to query a child
    ///   [`node`] by `index`. Returns a [`builder`] for a new child [`node`],
    ///   specified by `index`, for the resulting [`node`].
    ///
    /// This `builder` is used in a [`tree`] creation via the [`Tree::new()`]
    /// method and as a return value for the [`query`] callback.
    ///
    /// ---
    ///
    /// See the [`builder`] documentation for more information.
    ///
    /// [`node`]: crate::Node
    /// [`nodes`]: crate::Node
    /// [`cursor`]: crate::Cursor
    /// [`builder`]: crate::node::Builder
    /// [`query`]: crate::callbacks::Query
    /// [`link`]: crate::node::Link
    /// [`tree`]: crate::Tree
    /// [`Tree::new()`]: crate::Tree::new()
    pub fn new(
        value: impl Into<Value>,
        total: Total<Value, Error>,
        query: Query<Value, Error>,
    ) -> Self {
        let value = value.into();

        Self {
            parent: None,
            value,
            callbacks: Callbacks { total, query },
        }
    }

    /// # Allocates a [`node`] with specified configuration and returns a [`link`] to it
    ///
    /// Intentionally made private to prevent memory leaks due to incorrect use.
    ///
    /// ## Safety
    ///
    /// [`Node`] without a parent is considered `root` and requires manual
    /// removal using [`drop_link()`].
    ///
    /// [`Tree`] is allowed to have only one [`node`] without a parent.
    ///
    /// ---
    ///
    /// See the [`builder`] documentation for more information.
    ///
    /// [`node`]: crate::Node
    /// [`link`]: crate::node::Link
    /// [`Tree`]: crate::Tree
    /// [`builder`]: crate::node::Builder
    pub(crate) fn build(self) -> Link<Value, Error> {
        #[cfg(test)]
        {
            use crate::tree::NODES_ALIVE;
            use std::sync::atomic::Ordering;
            NODES_ALIVE.fetch_add(1, Ordering::Relaxed);
        }

        let itself = unsafe {
            NonNull::new_unchecked(Box::into_raw(Pin::into_inner_unchecked(
                Box::pin(MaybeUninit::<Node<Value, Error>>::uninit()),
            )) as *mut Node<Value, Error>)
        };

        let node = Node {
            parent: self.parent,
            children: Mutex::new(HashMap::new()),
            total: None,

            in_use: Mutex::new(0),

            value: self.value,
            callbacks: self.callbacks,
        };

        unsafe {
            ptr::write(itself.as_ptr(), node);
        }

        itself
    }
}

/// # Lazily-created `node`
///
/// ![](https://github.com/mnmun/images/blob/main/cherry.png?raw=true)
///
/// Structural unit of a [`tree`] that is created on an as-needed basis using
/// the [`query`] callback.
///
/// `Node` holds a `value` accessible via a [`cursor`], an optional [`link`] to
/// a parent [`node`], [`links`] to child [`nodes`], and two [callbacks]:
///
/// - `total` - calculates the `total` number of child [`nodes`] for this
///   [`node`];
/// - `query` - returns a [`builder`] for a new child [`node`], specified by
///   `index`, for this [`node`].
///
/// `Nodes` are connected through [`links`] - [`non-null`] pointers to
/// heap-allocated [`nodes`]. [`Links`] are used both to connect to an optional
/// parent [`node`] and to connect to child [`nodes`].
///
/// An important component of each [`node`] in the [`tree`] is an internal
/// `in_use` counter, which indicates how many [`cursors`] are currently
/// utilizing that [`node`]. In other words, this counter shows how many
/// [`cursors`] are, at any given moment, located at this [`node`] or below it
/// within the same branch of the [`tree`].
///
/// As soon as a [`cursor`] descends from a parent [`node`] to its child
/// [`node`], the `in_use` counter for that child [`node`] is **incremented** by
/// one. If this is the first [`cursor`] to descend into this [`node`] (the
/// `in_use` counter value before incrementing is equal to zero), then the
/// [`Total`] callback is called for the child [`node`] before incrementing the
/// counter.
///
/// As the [`cursor`] ascends from a child [`node`] to its parent [`node`], the
/// child [`node's`] `in_use` counter is **decremented** by one. If this is the
/// last [`cursor`] that was using the child [`node`] (the `in_use` counter
/// value became zero after decrementing), then this child [`node`] is
/// automatically deleted.
///
/// Thus, at any given moment, there exists a continuous sequence of [`nodes`]
/// connecting any [`node`] pointed to by the [`cursor`] to the `root` [`node`].
///
/// ## Creation
///
/// New [`node`] can be created using a [`builder`], however the `build()`
/// method is intentionally kept private to prevent the creation of orphan
/// [`nodes`] in cases where a [`link`] to a parent [`node`] might be forgotten.
/// Whenever a new [`node`] needs to be created, the functions accept configured
/// [`builders`] and automatically assign the correct parent [`link`], see the
/// [`Tree::new()`] method and the [`query`] callback.
///
/// ## Children
///
/// Child [`nodes`] are stored in the `HashMap<usize, Link<Value, Error>>` after
/// creation using the [`query`] callback. In the map the child [`node's`]
/// ordinal `index` serves as the `key`, and the `value` is a [`link`] to a
/// heap allocated child [`node`].
///
/// ---
///
/// See the [`module`] documentation for more information.
///
/// [`module`]: crate::node
/// [`tree`]: crate::Tree
/// [`query`]: crate::callbacks::Query
/// [`cursor`]: crate::Cursor
/// [`cursors`]: crate::Cursor
/// [`link`]: crate::node::Link
/// [`links`]: crate::node::Link
/// [`node`]: crate::Node
/// [`node's`]: crate::Node
/// [`nodes`]: crate::Node
/// [callbacks]: crate::callbacks
/// [`builder`]: crate::node::Builder
/// [`builders`]: crate::node::Builder
/// [`non-null`]: std::ptr::NonNull
/// [`Tree::new()`]: crate::Tree::new()
pub struct Node<Value, Error> {
    pub(crate) parent: Option<Link<Value, Error>>,
    pub(crate) children: Mutex<HashMap<usize, Link<Value, Error>>>,
    pub(crate) total: Option<usize>,

    pub(crate) in_use: Mutex<usize>,

    pub(crate) value: Value,
    pub(crate) callbacks: Callbacks<Value, Error>,
}

impl<Value, Error> Debug for Node<Value, Error>
where
    Value: fmt::Debug,
    Error: fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("parent", &self.parent)
            .field("children", &self.children)
            .field("in_use", &self.in_use)
            .field("value", &self.value)
            .field("callbacks", &self.callbacks)
            .finish()
    }
}

impl<Value, Error> Node<Value, Error> {
    /// # [`Cursor`] descends from the parent [`node`] to this child [`node`]
    ///
    /// Automatically called when the [`cursor`] moves from the parent [`node`]
    /// to this child [`node`], see the [`Cursor::down()`] method.
    ///
    /// This method first checks the current value of the [`node's`] `in_use`
    /// counter. If the counter value is zero, the [`total`] callback is invoked
    /// for this [`node`]. Regardless of the current `in_use` value, it is
    /// **incremented** by one to indicate that a new [`cursor`] is now using
    /// this [`node`].
    ///
    /// The antagonist method is the [`leave()`] method.
    ///
    /// ## Errors
    ///
    /// If the [`total`] callback completes its execution with an error, the
    /// `in_use` counter is not incremented and the [`cursor`] is not moved to
    /// this [`node`].
    ///
    /// ---
    ///
    /// See the [`node`] documentation for more information.
    ///
    /// [`node`]: crate::Node
    /// [`node's`]: crate::Node
    /// [`cursor`]: crate::Cursor
    /// [`Cursor`]: crate::Cursor
    /// [`Cursor::down()`]: crate::Cursor::down()
    /// [`total`]: crate::callbacks::Total
    /// [`leave()`]: crate::Node::leave()
    pub(crate) fn enter(&mut self) -> Result<(), Error> {
        let mut in_use = match self.in_use.lock() {
            Ok(guard) => guard,
            Err(poison) => poison.into_inner(),
        };

        if *in_use == 0 {
            self.total = Some((self.callbacks.total)(&self.value)?);
        }

        *in_use += 1;

        Ok(())
    }

    /// # [`Cursor`] ascends from this child [`node`] to its parent [`node`]
    ///
    /// Automatically called when the [`cursor`] is moved from this child
    /// [`node`] to its parent [`node`], see the [`Cursor::up()`] method.
    ///
    /// This method **decrements** the `in_use` counter of this child [`node`]
    /// by one. If the counter value reaches zero, all of this child [`node's`]
    /// own child [`nodes`] are deleted.
    ///
    /// The antagonist method is the [`enter()`] method.
    ///
    /// ---
    ///
    /// See the [`node`] documentation for more information.
    ///
    /// [`node`]: crate::Node
    /// [`node's`]: crate::Node
    /// [`nodes`]: crate::Node
    /// [`cursor`]: crate::Cursor
    /// [`Cursor`]: crate::Cursor
    /// [`Cursor::up()`]: crate::Cursor::up()
    /// [`enter()`]: crate::Node::enter()
    pub(crate) fn leave(&mut self) {
        let mut in_use = match self.in_use.lock() {
            Ok(guard) => guard,
            Err(poison) => poison.into_inner(),
        };

        *in_use = in_use
            .checked_sub(1)
            .expect("`in_use` counter could not be zero at this point");

        if *in_use == 0 {
            let mut children = match self.children.lock() {
                Ok(guard) => guard,
                Err(poison) => poison.into_inner(),
            };

            for (_, child) in mem::take(&mut *children) {
                drop_link(child);
            }
        }
    }

    /// # Attempts to delete a child [`node`] left by [`cursor`]
    ///
    /// After the [`cursor`] moves from a child [`node`] to this parent [`node`]
    /// (see the [`Cursor::up()`] method), it checks the `in_use` counter of
    /// that child [`node`]. If the counter is zero, it deletes that child
    /// [`node`].
    ///
    /// ---
    ///
    /// See the [`node`] documentation for more information.
    ///
    /// [`node`]: crate::Node
    /// [`cursor`]: crate::Cursor
    /// [`Cursor::up()`]: crate::Cursor::up()
    pub(crate) fn cleanup(&mut self, index: usize) {
        let mut children = match self.children.lock() {
            Ok(guard) => guard,
            Err(poison) => poison.into_inner(),
        };

        if let Entry::Occupied(entry) = children.entry(index)
            && *match unsafe { entry.get().as_ref().in_use.lock() } {
                Ok(guard) => guard,
                Err(poison) => poison.into_inner(),
            } == 0
        {
            drop_link(entry.remove());
        }
    }
}

#[cfg(test)]
impl<Value, Error> Node<Value, Error> {
    pub fn in_use(&self) -> MutexGuard<'_, usize> {
        match self.in_use.lock() {
            Ok(guard) => guard,
            Err(poison) => poison.into_inner(),
        }
    }

    pub fn children(&self) -> HashMap<usize, &Node<Value, Error>> {
        let children = match self.children.lock() {
            Ok(guard) => guard,
            Err(poison) => poison.into_inner(),
        };

        children
            .iter()
            .map(|(i, child)| (*i, unsafe { child.as_ref() }))
            .collect()
    }

    pub fn from_link<'a>(link: Link<Value, Error>) -> &'a Self {
        unsafe { link.as_ref() }
    }
}

impl<Value, Error> Drop for Node<Value, Error> {
    fn drop(&mut self) {
        let mut children = match self.children.lock() {
            Ok(guard) => guard,
            Err(poison) => poison.into_inner(),
        };

        for (_, child) in mem::take(&mut *children) {
            drop_link(child);
        }

        #[cfg(test)]
        {
            use crate::tree::NODES_ALIVE;
            use std::sync::atomic::Ordering;

            loop {
                let current = NODES_ALIVE.load(Ordering::Relaxed);

                match current.checked_sub(1) {
                    Some(new) => match NODES_ALIVE.compare_exchange(
                        current,
                        new,
                        Ordering::Release,
                        Ordering::Relaxed,
                    ) {
                        Ok(_) => break,
                        Err(_) => continue,
                    },
                    None => {
                        panic!(
                            "You forgot to increment the `NODES_ALIVE` counter when creating a node"
                        )
                    }
                }
            }
        }
    }
}
