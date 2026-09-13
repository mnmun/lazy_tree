//! # [`Tree`] of lazily populated [`nodes`]
//!
//! This module defines a [`tree`] - the owner of a `root` [`node`].
//!
//! See [`crate`] for more information.
//!
//! [`node`]: crate::Node
//! [`nodes`]: crate::Node
//! [`tree`]: Tree
use std::fmt::Debug;

use crate::{
    Cursor,
    node::{Link, drop_link},
};

#[cfg(feature = "debug")]
use std::sync::atomic::{AtomicUsize, Ordering};

/// # Counts live [`nodes`]
///
/// The counter is incremented by [`Builder::build()`] when a [`node`] is
/// allocated and decremented by [`Node::drop()`]. [`Tree::drop()`] checks that
/// the counter is zero after releasing the [`tree`] `root`; a non-zero value
/// indicates that one or more [`nodes`] were leaked.
///
/// [`node`]: crate::Node
/// [`nodes`]: crate::Node
/// [`Builder::build()`]: crate::Builder::build()
/// [`Node::drop()`]: crate::Node::drop()
/// [`tree`]: crate::Tree
/// [`Tree::drop()`]: crate::Tree::drop()
#[cfg(feature = "debug")]
pub(crate) static NODES_ALIVE: AtomicUsize = AtomicUsize::new(0);

/// ![tree](https://github.com/mnmun/images/blob/main/tree.png?raw=true)
///
/// # `Tree`
///
/// `Tree` holds a [`link`] to the `root` [`node`] and can be created using
/// [`Tree::new()`].
///
/// See [`crate`] for more information.
///
/// [`link`]: Link
/// [`node`]: crate::Node
pub struct Tree<'source, Value, Source, Error>
where
    Value: ToOwned + ?Sized + 'source,
    Source: Clone + 'source,
    [Source]: ToOwned<Owned = Vec<Source>>,
{
    pub(crate) root: Link<'source, Value, Source, Error>,
}

unsafe impl<'source, Value, Source, Error> Sync
    for Tree<'source, Value, Source, Error>
where
    Value: ToOwned + ?Sized + 'source,
    Source: Clone + 'source,
    [Source]: ToOwned<Owned = Vec<Source>>,
{
}

impl<'source, Value, Source, Error> Debug
    for Tree<'source, Value, Source, Error>
where
    Value: ToOwned + ?Sized + 'source,
    Source: Clone + 'source,
    [Source]: ToOwned<Owned = Vec<Source>>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tree").field("root", &self.root).finish()
    }
}

impl<'tree, 'source, Value, Source, Error> Tree<'source, Value, Source, Error>
where
    Value: ToOwned + ?Sized + 'source,
    Source: Clone + 'source,
    [Source]: ToOwned<Owned = Vec<Source>>,
{
    /// Creates a [`tree`] using [`link`] to the `root` [`node`]
    ///
    /// [`tree`]: Tree
    /// [`link`]: Link
    /// [`node`]: crate::Node
    pub fn new(root: impl Into<Link<'source, Value, Source, Error>>) -> Self {
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
    pub fn cursor(
        &'tree self,
    ) -> Result<Cursor<'tree, 'source, Value, Source, Error>, Error> {
        Cursor::new(self.root, vec![0])
    }
}

impl<'source, Value, Source, Error> Drop for Tree<'source, Value, Source, Error>
where
    Value: ToOwned + ?Sized + 'source,
    Source: Clone + 'source,
    [Source]: ToOwned<Owned = Vec<Source>>,
{
    /// Deallocates the entire [`tree`]
    ///
    /// [`tree`]: Tree
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
