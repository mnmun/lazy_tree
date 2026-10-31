//! # [`Node`] callbacks
//!
//! ![](https://github.com/mnmun/images/blob/main/telephone.png?raw=true)
//!
//! Each [`node`] in the [`tree`] contains two callbacks, which are invoked
//! automatically by the [`cursor`] as it moves through the [`tree`]:
//!
//! - [`Total`] - invoked when the [`cursor`] moves to a [`node`]. It ensures
//!   the [`Query`] callback functions correctly by calculating the `total`
//!   number of child [`nodes`] for a given parent [`node`]. If this calculation
//!   has already been performed for this [`node`], it is not repeated;
//! - [`Query`] - invoked when the [`cursor`] moves downward to a child [`node`]
//!   that has not yet been created. It handles the creation of a new child
//!   [`node`] only when access to that [`node`] is actually required.
//!
//! In this way, a developer can define the logic for constructing a [`tree`] of
//! lazily-created [`nodes`] tailored to their specific use case by implementing
//! these callbacks.
//!
//! ---
//!
//! See the [`crate`] documentation for more information.
//!
//! [`lazy_tree`]: crate
//! [`tree`]: crate::Tree
//! [`Node`]: crate::Node
//! [`node`]: crate::Node
//! [`nodes`]: crate::Node
//! [`cursor`]: crate::Cursor

use crate::node;

/// # Callback for the `total` number of child [`nodes`]
///
/// Calculates the `total` number of child [`nodes`] for the [`node`] that holds
/// this callback, based on a shared reference to the `value` stored within that
/// [`node`].
///
/// This callback is called automatically by the [`cursor`] when it moves to a
/// [`node`], provided it has not been called previously.
///
/// The `total` number of child [`nodes`] returned for a given parent [`node`]
/// will be used to define the range of valid child [`node`] indices -
/// `0..total`, such that any attempt to access a child [`node`] with an index
/// outside of this range will be considered invalid.
///
/// ---
///
/// See the [`module`] documentation for more information.
///
/// [`module`]: crate::callbacks
/// [`cursor`]: crate::Cursor
/// [`node`]: crate::Node
/// [`nodes`]: crate::Node
pub type Total<Value, Error> = fn(parent: &Value) -> Result<usize, Error>;

/// # Callback to query a child [`node`] by `index`
///
/// Returns a [`builder`] for a new child [`node`], specified by `index`, for a
/// [`node`] that holds this callback.
///
/// The callback receives the following arguments:
///
/// - `parent` - a shared reference to the `value` stored in the parent
///   [`node`];
/// - `index` - the `index` of queried [`node`].
///
/// This callback is automatically invoked on a parent [`node`] whenever the
/// [`cursor`] needs to move from the parent [`node`] to a child [`node`]
/// identified by a given `index`, that has not yet been created.
///
/// It is guaranteed that the `index` will fall within the range `0..total`,
/// where `total` is a value obtained from the parent [`node's`] [`Total`]
/// callback.
///
/// ---
///
/// See the [`module`] documentation for more information.
///
/// [`module`]: crate::callbacks
/// [`node`]: crate::Node
/// [`node's`]: crate::Node
/// [`nodes`]: crate::Node
/// [`link`]: crate::node::Link
/// [`cursor`]: crate::Cursor
/// [`builder`]: crate::node::Builder
pub type Query<Value, Error> = fn(
    parent: &Value,
    index: usize,
)
    -> Result<node::Builder<Value, Error>, Error>;

#[derive(Debug)]
pub(crate) struct Callbacks<Value, Error> {
    pub(crate) total: Total<Value, Error>,
    pub(crate) query: Query<Value, Error>,
}
