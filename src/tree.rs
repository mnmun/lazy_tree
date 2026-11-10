//! # [`Tree`] of lazily-created [`nodes`]
//!
//! ![](https://github.com/mnmun/images/blob/main/tree.png?raw=true)
//!
//! Defines a [`tree`] - a wrapper around a [`NonNull`] pointer to the `root`
//! [`node`] with the ability to [create] [`cursors`] for traversing
//! hierarchical structure of lazily-created [`nodes`].
//!
//! ---
//!
//! See the [`crate`] documentation for more information.
//!
//! [`node`]: crate::Node
//! [`nodes`]: crate::Node
//! [`tree`]: crate::Tree
//! [`NonNull`]: std::ptr::NonNull
//! [create]: crate::Tree::cursor()
//! [`cursors`]: crate::Cursor
use std::{fmt, thread};

use tokio::sync::mpsc;

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

/// # Asynchronous task kinds
///
/// See the [`Tree::new()`] method documentation for more information.
pub(crate) enum Cleaning<Value, Error>
where
    Value: Send + 'static,
    Error: Send + 'static,
{
    Park {
        link: Link<Value, Error>,
        path: Vec<usize>,
    },
    Drop(Link<Value, Error>),
    Close,
}

/// # `Tree` of lazily-created [`nodes`]
///
/// ![](https://github.com/mnmun/images/blob/main/tree.png?raw=true)
///
/// Holds a [`NonNull`] pointer to the `root` [`node`] and provides a
/// [`cursor()`] method for creating new [`cursors`].
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
/// [`cursor()`]: crate::Tree::cursor()
/// [`cursor`]: crate::Cursor
/// [`cursors`]: crate::Cursor
/// [`Cursors`]: crate::Cursor
/// [`Tree::new()`]: crate::Tree::new()
/// [`builder`]: crate::node::Builder
/// [creation]: crate::node::Callbacks::query
/// [`Sync`]: std::marker::Sync
/// [`Send`]: std::marker::Send
/// [`NonNull`]: std::ptr::NonNull
pub struct Tree<Value, Error>
where
    Value: Send + 'static,
    Error: Send + 'static,
{
    /// # [`Link`] to the `root` [`node`]
    ///
    /// See the [`tree`] documentation for more information.
    ///
    /// [`Link`]: crate::node::Link
    /// [`node`]: crate::Node
    /// [`tree`]: crate::Tree
    pub(crate) root: Link<Value, Error>,

    /// # Communication channel with an asynchronous task for deleting the `root` [`node`]
    ///
    /// On [`tree`] `drop`, it sends a [`link`] to the `root` [`node`] for
    /// deleting to the channel.
    ///
    /// ---
    ///
    /// See the [`tree`] documentation for more information.
    ///
    /// [`tree`]: crate::Tree
    /// [`link`]: crate::node::Link
    /// [`node`]: crate::Node
    cleaning: mpsc::UnboundedSender<Cleaning<Value, Error>>,
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
unsafe impl<Value, Error> Sync for Tree<Value, Error>
where
    Value: Send + 'static,
    Error: Send + 'static,
{
}

/// # Safety
///
/// [`Tree`] can be safely sent between threads, since the borrow checked
/// ensures that the [`cursor`] cannot outlive the [`tree`] from which it was
/// created.
///
/// ---
///
/// See the [`tree`] documentation for more information.
///
/// [`tree`]: crate::Tree
/// [`Tree`]: crate::Tree
/// [`cursor`]: crate::Cursor
unsafe impl<Value, Error> Send for Tree<Value, Error>
where
    Value: Send + 'static,
    Error: Send + 'static,
{
}

impl<Value, Error> fmt::Debug for Tree<Value, Error>
where
    Value: Send + 'static,
    Error: Send + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tree")
            .field("root", &self.root)
            .finish_non_exhaustive()
    }
}

impl<'tree, Value, Error> Tree<Value, Error>
where
    Value: Send + 'static,
    Error: Send + 'static,
{
    /// # Creates a [`tree`] based on a `root` [`node`] [`builder`]
    ///
    /// See the [`tree`] documentation for more information.
    ///
    /// [`tree`]: crate::Tree
    /// [`node`]: crate::Node
    /// [`builder`]: crate::node::Builder
    pub fn new(builder: impl Into<node::Builder<Value, Error>>) -> Self {
        let (cleaning, mut receiver) =
            mpsc::unbounded_channel::<Cleaning<Value, Error>>();

        thread::spawn(move || {
            futures_lite::future::block_on(async move {
                while let Some(message) = receiver.recv().await {
                    match message {
                        Cleaning::Park { mut link, mut path } => unsafe {
                            while let Some(parent) = link.as_ref().parent {
                                link.as_mut().leave().await;
                                link = parent;
                                link.as_mut()
                                    .cleanup(path.pop().unwrap())
                                    .await;
                            }

                            link.as_mut().leave().await;
                        },
                        Cleaning::Drop(link) => {
                            drop_link(link).await;
                        }
                        Cleaning::Close => break,
                    }
                }

                #[cfg(test)]
                {
                    let remaining = NODES_ALIVE.load(Ordering::Acquire);

                    if remaining != 0 {
                        panic!(
                            "Memory leak! {remaining} nodes are still alive"
                        );
                    }
                }
            })
        });

        Self {
            root: builder.into().build(),
            cleaning,
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
    pub async fn cursor(
        &'tree self,
    ) -> Result<Cursor<'tree, Value, Error>, Error> {
        Cursor::new(self.root, vec![0], self.cleaning.clone()).await
    }
}

/// # Deallocates the entire [`tree`] of [`nodes`]
///
/// Keep in mind that the removal of [`nodes`] might not be completed
/// immediately.
///
/// ---
///
/// See the [`tree`] documentation for more information.
///
/// [`tree`]: crate::Tree
/// [`nodes`]: crate::Node
impl<Value, Error> Drop for Tree<Value, Error>
where
    Value: Send + 'static,
    Error: Send + 'static,
{
    fn drop(&mut self) {
        let _ = self.cleaning.send(Cleaning::Drop(self.root));
        let _ = self.cleaning.send(Cleaning::Close);
    }
}
