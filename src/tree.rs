//! # [`Tree`] of lazily populated [`nodes`]
//!
//! ![](https://github.com/mnmun/images/blob/main/tree.png?raw=true)
//!
//! Defines a [`tree`] - the owner of a `root` [`node`].
//!
//! ---
//!
//! See the [`crate documentation`] for more information.
//!
//! [`node`]: crate::Node
//! [`nodes`]: crate::Node
//! [`tree`]: Tree
//! [`crate documentation`]: crate
use std::fmt::Debug;

use crate::{
    Cursor,
    node::{Link, drop_link},
};

#[cfg(feature = "debug")]
use std::sync::atomic::{AtomicUsize, Ordering};

/// # Counts live [`nodes`]
///
/// The counter is incremented by [`Node::new()`] when a [`node`] is allocated
/// and decremented by [`Node::drop()`]. [`Tree::drop()`] checks that the
/// counter is zero after releasing the [`tree`] `root`; a non-zero value
/// indicates that one or more [`nodes`] were leaked.
///
/// [`node`]: crate::Node
/// [`nodes`]: crate::Node
/// [`Node::new()`]: crate::Node::new()
/// [`Node::drop()`]: crate::Node::drop()
/// [`tree`]: crate::Tree
/// [`Tree::drop()`]: crate::Tree::drop()
#[cfg(feature = "debug")]
pub(crate) static NODES_ALIVE: AtomicUsize = AtomicUsize::new(0);

/// # `Tree` of lazily populated [`nodes`]
///
/// ![](https://github.com/mnmun/images/blob/main/tree.png?raw=true)
///
/// `Tree` holds a [`link`] to the `root` [`node`] and can be created using
/// the [`Tree::new()`] method.
///
/// The `tree` of [`nodes`] can be traversed using [`cursors`] created via the
/// [`cursor()`] method. There is no limit to the number of [`cursors`] for the
/// `tree` and keep in mind that a [`cursor`] cannot outlive the `tree` from
/// which it was created.
///
/// `Tree` [`nodes`] are populated and released lazily by [`cursors`]. See the
/// "Children" section in the [`node documentation`] for more information.
///
/// ---
///
/// See the [`module documentation`] for more information.
///
/// [`nodes`]: crate::Node
/// [`link`]: Link
/// [`node`]: crate::Node
/// [`cursor()`]: Tree::cursor()
/// [`cursor`]: Cursor
/// [`cursors`]: Cursor
/// [`node documentation`]: crate::Node
/// [`module documentation`]: crate::tree
pub struct Tree<Value, Error> {
    pub(crate) root: Link<Value, Error>,
}

unsafe impl<Value, Error> Sync for Tree<Value, Error> {}

impl<Value, Error> Debug for Tree<Value, Error> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tree").field("root", &self.root).finish()
    }
}

impl<'tree, Value, Error> Tree<Value, Error> {
    /// # Creates a [`tree`] using [`link`] to the `root` [`node`]
    ///
    /// [`tree`]: Tree
    /// [`link`]: Link
    /// [`node`]: crate::Node
    pub fn new(root: impl Into<Link<Value, Error>>) -> Self {
        Self { root: root.into() }
    }

    /// # Creates a [`cursor`] at the [`tree's`] `root` [`node`]
    ///
    /// The returned [`cursor`] is positioned at the [`tree's`] `root` [`node`]
    /// and has [`path`] set to `[0]`.
    ///
    /// [`cursor`]: Cursor
    /// [`tree's`]: Tree
    /// [`node`]: crate::Node
    /// [`path`]: Cursor::path()
    pub fn cursor(&'tree self) -> Result<Cursor<'tree, Value, Error>, Error> {
        Cursor::new(self.root, vec![0])
    }
}

impl<Value, Error> Drop for Tree<Value, Error> {
    fn drop(&mut self) {
        drop_link(self.root);

        #[cfg(feature = "debug")]
        {
            let remaining = NODES_ALIVE.load(Ordering::Acquire);

            if remaining != 0 {
                panic!("Memory leak! {remaining} nodes are still alive");
            }
        }
    }
}
