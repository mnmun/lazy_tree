//! # [`Tree`] of lazily-created [`nodes`]
//!
//! ![](https://github.com/mnmun/images/blob/main/tree.png?raw=true)
//!
//! Defines a [`tree`] - a wrapper around the [`link`] to the `root` [`node`]
//! with the ability to [create] [`cursors`] for traversing hierarchical
//! structure of lazily-created [`nodes`].
//!
//! ---
//!
//! See the [`crate`] documentation for more information.
//!
//! [`node`]: crate::Node
//! [`nodes`]: crate::Node
//! [`tree`]: crate::Tree
//! [`link`]: crate::node::Link
//! [create]: crate::Tree::cursor()
//! [`cursors`]: crate::Cursor
use std::fmt::Debug;

use crate::{
    Cursor,
    node::{self, Link, drop_link},
};

#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

/// # Counts live [`nodes`]
///
/// The counter is incremented when a [`node`] is created using the
/// [`Builder::build()`] method and decremented when a [`node`] is deleted via
/// the [`Node::drop()`] method. The [`Tree::drop()`] method verifies that the
/// counter is equal to zero after the [`tree's`] `root` [`node`] has been
/// deleted; a non-zero counter value indicates a memory leak - meaning some
/// [`nodes`] were not properly removed.
///
/// ---
///
/// See the [`node`] documentation for more information.
///
/// [`node`]: crate::Node
/// [`nodes`]: crate::Node
/// [`Builder::build()`]: crate::node::Builder::build()
/// [`Node::drop()`]: crate::Node::drop()
/// [`tree`]: crate::Tree
/// [`tree's`]: crate::Tree
/// [`Tree::drop()`]: crate::Tree::drop()
#[cfg(test)]
pub(crate) static NODES_ALIVE: AtomicUsize = AtomicUsize::new(0);

/// # `Tree` of lazily-created [`nodes`]
///
/// ![](https://github.com/mnmun/images/blob/main/tree.png?raw=true)
///
/// Holds a [`link`] to the `root` [`node`] and provides a [`cursor()`] method
/// for creating new [`cursors`].
///
/// `Tree` implements the [`Sync`] and [`Send`] traits, which allow a single
/// `tree` to be shared among multiple threads, as well as sent between
/// threads.
///
/// ## Creation
///
/// Can be created using the [`Tree::new()`] method, which takes the `root`
/// [`node`] [`builder`] as an argument.
///
/// ## Traversing
///
/// Traversing a `tree` of lazily-created [`nodes`] is performed using
/// [`cursors`], which move between [`nodes`] and trigger the [creation] and
/// deletion of `tree` [`nodes`]. A [`cursor`] provides access to the `value`
/// stored in the [`node`] it currently points to.
///
/// `Tree` may have multiple [`cursors`] at the same time, pointing to
/// different [`nodes`] and moving independently of each other. [`Cursors`]
/// implement the [`Send`] trait, so they can be sent between threads safely.
/// Each [`cursor`] remains borrowed from its `tree` and cannot outlive it.
///
/// ---
///
/// See the [`module`] documentation for more information.
///
/// [`module`]: crate::tree
/// [`node`]: crate::Node
/// [`nodes`]: crate::Node
/// [`link`]: crate::node::Link
/// [`cursor()`]: crate::Tree::cursor()
/// [`cursor`]: crate::Cursor
/// [`cursors`]: crate::Cursor
/// [`Cursors`]: crate::Cursor
/// [`Tree::new()`]: crate::Tree::new()
/// [`builder`]: crate::node::Builder
/// [creation]: crate::callbacks::Query
/// [`Sync`]: std::marker::Sync
/// [`Send`]: std::marker::Send
pub struct Tree<Value, Error> {
    pub(crate) root: Link<Value, Error>,
}

/// # Safety
///
/// [`Tree`] contains no elements that would prevent it from being shared
/// among multiple threads.
///
/// ---
///
/// See the [`tree`] documentation for more information.
///
/// [`tree`]: crate::Tree
/// [`Tree`]: crate::Tree
unsafe impl<Value, Error> Sync for Tree<Value, Error> {}

/// # Safety
///
/// [`Tree`] can be safely sent between threads, since the borrow checked
/// ensures that the cursor cannot outlive the tree from which it was created.
///
/// ---
///
/// See the [`tree`] documentation for more information.
///
/// [`tree`]: crate::Tree
/// [`Tree`]: crate::Tree
unsafe impl<Value, Error> Send for Tree<Value, Error> {}

impl<Value, Error> Debug for Tree<Value, Error> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tree").field("root", &self.root).finish()
    }
}

impl<'tree, Value, Error> Tree<Value, Error> {
    /// # Creates a [`tree`] based on a `root` [`node`] [`builder`]
    ///
    /// See the [`tree`] documentation for more information.
    ///
    /// [`tree`]: crate::Tree
    /// [`node`]: crate::Node
    /// [`builder`]: crate::node::Builder
    pub fn new(builder: impl Into<node::Builder<Value, Error>>) -> Self {
        Self {
            root: builder.into().build(),
        }
    }

    /// # Creates a [`cursor`] at the [`tree`] `root` [`node`]
    ///
    /// The returned [`cursor`] is positioned at the `root` [`node`] with
    /// [`path`] set to `[0]`.
    ///
    /// ---
    ///
    /// See the [`tree`] documentation for more information.
    ///
    /// [`cursor`]: crate::Cursor
    /// [`tree`]: crate::Tree
    /// [`node`]: crate::Node
    /// [`path`]: crate::Cursor::path()
    pub fn cursor(&'tree self) -> Result<Cursor<'tree, Value, Error>, Error> {
        Cursor::new(self.root, vec![0])
    }
}

impl<Value, Error> Drop for Tree<Value, Error> {
    fn drop(&mut self) {
        drop_link(self.root);

        #[cfg(test)]
        {
            let remaining = NODES_ALIVE.load(Ordering::Acquire);

            if remaining != 0 {
                panic!("Memory leak! {remaining} nodes are still alive");
            }
        }
    }
}
