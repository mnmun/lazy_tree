//! # Lazily populated [`node`]
//!
//! This module contains the representation of a lazily populated [`node`].
//!
//! See [`crate`] for more information.
//!
//! [`node`]: Node
use std::{
    borrow::Cow, cell::UnsafeCell, fmt::Debug, ops::Range, ptr::NonNull,
    sync::Mutex,
};

use getset::{Getters, MutGetters};

/// # [`NonNull`] pointer to a [`node`]
///
/// `Link` is used to connect a [`node`] with its parent and children.
///
/// See [`crate`] for more information.
///
/// [`node`]: Node
pub type Link<'source, Value, Source, Error> =
    NonNull<Node<'source, Value, Source, Error>>;

/// ![populate](https://github.com/mnmun/images/blob/main/maternity.png?raw=true)
///
/// # Callback used to create a [`node's`] children
///
/// Invoked when a [`cursor`] enters a [`node`] whose children have not yet been
/// populated. This occurs when the [`node`] has no other [`cursors`] visiting
/// it at this moment.
///
/// The callback receives:
///
/// - `source`: the source data;
/// - `range`: an optional metadata describing the relevant source range;
/// - `parent`: a [`link`] to the parent [`node`].
///
/// On success, it returns a boxed slice containing [`links`] to the created
/// children. Otherwise, it returns an `error`.
///
/// ## Example
///
/// The following example shows a simple `populate` function that can be used
/// as a callback for a [`node`]. The function creates two children and assigns
/// their values as "left" and "right". In this illustrative example `source`
/// and `range` are not used just to keep example simple.
///
/// ```no_run
/// use std::{ops::Range, borrow::Cow};
///
/// use lazy_tree::node::{Link, Builder};
///
/// type MyValue = str;
/// type MySource = (); // `source` is unused, so its element type is `()`
///
/// enum MyError {
///     PopulationFailed,
/// };
///
/// // A type alias for `Link` with the type parameters specified
/// type MyLink<'source> = Link<'source, MyValue, MySource, MyError>;
///
/// // A type alias for a boxed slice of `MyLink`
/// type Children<'source> = Box<[MyLink<'source>]>;
///
/// fn populate<'source>(
///     source: impl Into<Cow<'source, [MySource]>>,
///     range: impl Into<Option<Range<usize>>>,
///     parent: MyLink<'source>,
/// ) -> Result<Children<'source>, MyError> {
///
///     // Callback can return user-defined error if population may fail
///     // if some_condition {
///     //     return Err(MyError::PopulationFailed)
///     // }
///
///     let source = source.into();
///
///     let left_child = Builder::new("left", source.clone(), populate).build();
///     let right_child = Builder::new("right", source, populate).build();
///
///     Ok(Box::new([left_child, right_child]))
/// }
/// ```
///
/// See [`crate`] for more information.
///
/// [`node`]: Node
/// [`node's`]: Node
/// [`cursor`]: crate::Cursor
/// [`cursors`]: crate::Cursor
/// [`link`]: Link
/// [`links`]: Link
pub type Populate<'source, Value, Source, Error> =
    fn(
        Cow<'source, [Source]>,
        Option<Range<usize>>,
        Link<'source, Value, Source, Error>,
    ) -> Result<Box<[Link<'source, Value, Source, Error>]>, Error>;

/// Deallocates the subtree rooted at [`link`]
///
/// [`link`]: Link
pub(crate) fn drop_link<'source, Value, Source, Error>(
    link: Link<'source, Value, Source, Error>,
) where
    Value: ToOwned + ?Sized + 'source,
    Source: Clone + 'source,
    [Source]: ToOwned<Owned = Vec<Source>>,
{
    let mut delete_queue = vec![link];
    while let Some(mut node) = delete_queue.pop() {
        let node = unsafe { Box::from_raw(node.as_mut()) };
        let children = unsafe { node.children.get().replace(Box::default()) };
        delete_queue.extend(children);
    }
}

/// # [`Node`] builder
///
/// Data required to build a [`node`]:
///
/// - `value`: the data associated with this node and exposed by
///   [`Node::value()`];
/// - `source`: the input collection used in the [`population callback`] for
///   children creation;
/// - `populate`: a [`callback`] used to create the [`node's`] children.
///
/// Optional [`node`] data:
///
/// - `parent`: a [`link`] to the parent [`node`], used for upward traversal;
/// - `range`: metadata that passed to [`population callback`] and might be used
///   to describe the part of `source` collection associated with this [`node`].
///
/// After setting all the necessary fields, call [`build()`] to get a [`link`]
/// to the [`node`] initialized with the specified data.
///
/// See [`crate`] for more information.
///
/// [`node`]: Node
/// [`node's`]: Node
/// [`callback`]: Populate
/// [`population callback`]: Populate
/// [`link`]: Link
/// [`build()`]: Builder::build()
#[derive(Getters, MutGetters)]
#[getset(get = "pub", get_mut = "pub")]
pub struct Builder<'source, Value, Source, Error>
where
    Value: ToOwned + ?Sized + 'source,
    Source: Clone + 'source,
    [Source]: ToOwned<Owned = Vec<Source>>,
{
    parent: Option<Link<'source, Value, Source, Error>>,
    range: Option<Range<usize>>,

    value: Cow<'source, Value>,
    source: Cow<'source, [Source]>,
    populate: Populate<'source, Value, Source, Error>,
}

impl<'source, Value, Source, Error> Clone
    for Builder<'source, Value, Source, Error>
where
    Value: ToOwned + ?Sized + 'source,
    Source: Clone + 'source,
    [Source]: ToOwned<Owned = Vec<Source>>,
{
    fn clone(&self) -> Self {
        Self {
            parent: self.parent,
            range: self.range.clone(),
            value: self.value.clone(),
            source: self.source.clone(),
            populate: self.populate,
        }
    }
}

impl<'source, Value, Source, Error> Builder<'source, Value, Source, Error>
where
    Value: ToOwned + ?Sized + 'source,
    Source: Clone + 'source,
    [Source]: ToOwned<Owned = Vec<Source>>,
{
    #[must_use = "method returns the modified value"]
    pub fn with_parent(
        mut self,
        value: impl Into<Option<Link<'source, Value, Source, Error>>>,
    ) -> Self {
        self.parent = value.into();
        self
    }

    #[must_use = "method returns the modified value"]
    pub fn with_range(
        mut self,
        value: impl Into<Option<Range<usize>>>,
    ) -> Self {
        self.range = value.into();
        self
    }
}

impl<'source, Value, Source, Error> Builder<'source, Value, Source, Error>
where
    Value: ToOwned + ?Sized + 'source,
    Source: Clone + 'source,
    [Source]: ToOwned<Owned = Vec<Source>>,
{
    pub fn set_parent(
        &mut self,
        value: impl Into<Option<Link<'source, Value, Source, Error>>>,
    ) -> &mut Self {
        self.parent = value.into();
        self
    }

    pub fn set_range(
        &mut self,
        value: impl Into<Option<Range<usize>>>,
    ) -> &mut Self {
        self.range = value.into();
        self
    }
}

impl<'source, Value, Source, Error> Builder<'source, Value, Source, Error>
where
    Value: ToOwned + ?Sized + 'source,
    Source: Clone + 'source,
    [Source]: ToOwned<Owned = Vec<Source>>,
{
    /// # Creates a new [`builder`]
    ///
    /// Fields `parent` and `range` are set to `None` by default.
    ///
    /// [`builder`]: Builder
    pub fn new(
        value: impl Into<Cow<'source, Value>>,
        source: impl Into<Cow<'source, [Source]>>,
        populate: Populate<'source, Value, Source, Error>,
    ) -> Self {
        let value = value.into();
        let source = source.into();

        Self {
            parent: None,
            range: None,

            value,
            source,
            populate,
        }
    }

    /// Allocates a [`node`] consuming this [`builder`] and returns a [`link`]
    /// to it
    ///
    /// [`node`]: Node
    /// [`builder`]: Builder
    /// [`link`]: Link
    pub fn build(self) -> Link<'source, Value, Source, Error> {
        #[cfg(feature = "leak-detection")]
        {
            use crate::tree::NODES_ALIVE;
            use std::sync::atomic::Ordering;
            NODES_ALIVE.fetch_add(1, Ordering::Relaxed);
        }

        let node = Node {
            parent: self.parent,
            children: UnsafeCell::default(),
            visitors: Mutex::new(0),

            value: self.value,

            source: self.source,
            range: self.range,
            populate: self.populate,
        };

        unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(node))) }
    }
}

/// ![node](https://github.com/mnmun/images/blob/main/cherry.png?raw=true)
///
/// # `Node`
///
/// A `node` stores a [`value`] together with the `source` data and `range`
/// metadata needed to create its children. `Nodes` are connected through
/// [`links`] - [`non-null`] pointers to heap-allocated `nodes`. [`Links`] are
/// used both for the optional parent pointer and for pointers to currently
/// populated children.
///
/// ## Creation
///
/// `Node` instance can be obtained using [`builder`] only in the form of a
/// [`link`].
///
/// ## Children
///
/// `Node` has an internal counter that tracks how many [`cursors`] are
/// visiting it. When a [`cursor`] descends to a `node` (visits), its counter
/// is incremented; when a [`cursor`] ascends from the `node` (leaves) -
/// decremented. When a [`cursor`] visits a `node` whose current counter is
/// zero, the `node's` [`population callback`] is invoked. The resulting
/// children remain available until the last [`cursor`] leaves the `node`, at
/// which point the counter reaches zero and children are released.
///
/// See [`crate`] for more information.
///
/// [`non-null`]: NonNull
/// [`value`]: Node::value()
/// [`link`]: Link
/// [`links`]: Link
/// [`Links`]: Link
/// [`builder`]: Builder
/// [`cursors`]: crate::Cursor
/// [`cursor`]: crate::Cursor
/// [`population callback`]: Populate
pub struct Node<'source, Value, Source, Error>
where
    Value: ToOwned + ?Sized + 'source,
    Source: Clone + 'source,
    [Source]: ToOwned<Owned = Vec<Source>>,
{
    pub(crate) parent: Option<Link<'source, Value, Source, Error>>,
    pub(crate) children: UnsafeCell<Box<[Link<'source, Value, Source, Error>]>>,
    pub(crate) visitors: Mutex<usize>,

    value: Cow<'source, Value>,

    source: Cow<'source, [Source]>,
    range: Option<Range<usize>>,
    populate: Populate<'source, Value, Source, Error>,
}

impl<'source, Value, Source, Error> Debug
    for Node<'source, Value, Source, Error>
where
    Value: ToOwned + ?Sized + 'source,
    Source: Clone + 'source,
    [Source]: ToOwned<Owned = Vec<Source>>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("parent", &self.parent)
            .field("children", unsafe { &*self.children.get() })
            .field("visitors", &self.visitors)
            .finish()
    }
}

impl<'source, Value, Source, Error> Node<'source, Value, Source, Error>
where
    Value: ToOwned + ?Sized + 'source,
    Source: Clone + 'source,
    [Source]: ToOwned<Owned = Vec<Source>>,
{
    /// Releases all children of the [`node`]
    ///
    /// [`node`]: Node
    fn kill_children(&self) {
        for child in unsafe { self.children.get().replace(Box::default()) } {
            drop_link(child);
        }
    }

    /// # Populates the [`node's`] children using its [`callback`]
    ///
    /// Any previously populated children are released.
    ///
    /// [`node's`]: Node
    /// [`callback`]: Populate
    fn make_children(
        &self,
        link_to_itself: Link<'source, Value, Source, Error>,
    ) -> Result<(), Error> {
        let children = (self.populate)(
            self.source.clone(),
            self.range.clone(),
            link_to_itself,
        )?;

        for child in unsafe { self.children.get().replace(children) } {
            drop_link(child);
        }

        Ok(())
    }

    /// Convenient wrapper for an `unsafe` call
    pub(crate) fn from_link(
        link: Link<'source, Value, Source, Error>,
    ) -> &'source Self {
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
    /// [`population callback`] if previous value of `visitors` was zero.
    ///
    /// # Errors
    ///
    /// If the [`population callback`] returns an `error`, the `visitors`
    /// counter is not incremented, and the children are populated on the next
    /// successful attempt to visit this [`node`].
    ///
    /// [`cursor`]: crate::Cursor
    /// [`walk()`]: crate::Cursor::walk()
    /// [`jump()`]: crate::Cursor::jump()
    /// [`Direction::Down`]: crate::Direction::Down
    /// [`node`]: Node
    /// [`node's`]: Node
    /// [`population callback`]: Populate
    pub(crate) fn visit(
        &self,
        link_to_itself: Link<'source, Value, Source, Error>,
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

    /// Returns a reference to the [`node's`] `value`
    ///
    /// [`node's`]: Node
    pub fn value(&self) -> &Cow<'source, Value> {
        &self.value
    }

    /// Returns the `values` of all currently populated children
    pub fn children(&self) -> Box<[Cow<'source, Value>]> {
        unsafe {
            (*self.children.get())
                .iter()
                .map(|child| child.as_ref().value.clone())
                .collect()
        }
    }
}

impl<'source, Value, Source, Error> Drop for Node<'source, Value, Source, Error>
where
    Value: ToOwned + ?Sized + 'source,
    Source: Clone + 'source,
    [Source]: ToOwned<Owned = Vec<Source>>,
{
    fn drop(&mut self) {
        #[cfg(feature = "leak-detection")]
        {
            use crate::tree::NODES_ALIVE;
            use std::sync::atomic::Ordering;
            NODES_ALIVE.fetch_sub(1, Ordering::Relaxed);
        }
    }
}
