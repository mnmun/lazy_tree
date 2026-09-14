//! ![logo](https://github.com/mnmun/images/blob/main/tree.png?raw=true)
//!
//! # A [`tree`] of lazily populated [`nodes`] with [`cursor-based`] traversal
//!
//! This crate provides a [`cursor-based`] interface for traversing [`trees`]
//! without materializing the entire hierarchy in memory. The [`nodes`] are
//! populated lazily when visited by [`cursors`] and remain available only while
//! they are needed by active [`cursors`].
//!
//! ## Overview
//!
//! Each [`node`] has an internal counter that tracks how many [`cursors`] are
//! visiting it. When a [`cursor`] descends to a [`node`] (visits), its counter
//! is incremented; when a [`cursor`] ascends from the [`node`] (leaves) -
//! decremented. When a [`cursor`] visits a [`node`] whose current counter is
//! zero, the [`node's`] [`population callback`] is invoked. The resulting
//! children remain available until the last [`cursor`] leaves the [`node`], at
//! which point the counter reaches zero and children are released.
//!
//! A [`cursor`] can move in both axes through the [`tree`]:
//!
//! - Vertically, from a parent to one of its children and back;
//! - Horizontally, to the next or previous sibling, including siblings in an
//!   adjacent branch.
//!
//! The following diagram shows the example [`tree`] in various states
//! determined by the different positions of the [`cursor`], indicated by
//! asterisks:
//!
//! ```plain
//!                                               ROOT
//!                             ROOT            ┌──┼──┐
//!             *ROOT*        ┌──┼──┐           A  B  C
//!   ROOT     ┌──┼──┐       *A* B  C        ┌──┼──┐   
//!            A  B  C     ┌──┼──┐          *D* E  F   
//!   (0)                  D  E  F        ┌──┼──┐      
//!              (1)                      G  H  I      
//!                            (2)                     
//!                                             (3)    
//! ```
//!
//! Consider a single [`cursor`] descending through the [`tree`]
//! (`(0)` → `(3)`):
//!
//! - `(0)`: The [`tree`] has no [`cursors`], and only the `ROOT` [`node`] is
//!   present;
//! - `(1)`: A [`cursor`] is created at `ROOT`, and `ROOT's` children populated
//!   by a [`callback`];
//! - `(2)`: The [`cursor`] descends from `ROOT` to `A` and visits `A`. Because
//!   this is the first [`cursor`] to visit `A`, its children are populated by
//!   the [`callback`]. `ROOT` and its children remain available;
//! - `(3)`: The [`cursor`] descends from `A` to `D` and visits `D`. Because
//!   this is the first [`cursor`] to visit `D`, its children are populated by
//!   the [`callback`]. `A` and its children remain available.
//!
//! Thus, a [`cursor`] moving downward leaves behind a path of populated
//! [`nodes`] from the [`tree's`] `root` to its current position.
//!
//! Now consider the reverse traversal, in which the [`cursor`] ascends through
//! the [`tree`] (`(3)` → `(0)`):
//!
//! - `(3)`: The [`cursor`] is at `D`, whose children are currently populated;
//! - `(2)`: The [`cursor`] ascends from `D` to `A`; it leaves `D`, and its
//!   children are released because no other [`cursor`] keeps `D` visited;
//! - `(1)`: The [`cursor`] ascends from `A` to `ROOT`; it leaves `A`, and its
//!   children are released because no other [`cursor`] keeps `A` visited;
//! - `(0)`: The [`cursor`] is dropped, and all [`nodes`] except `ROOT` in the
//!   [`tree`] are released.
//!
//! In other words, [`nodes`] populated during traversal are released
//! automatically once no [`cursor`] keeps them visited.
//!
//! ## Types
//!
//! The crate provides the following main types:
//!
//! - [`Tree`] - owns the `root` [`node`] and creates [`cursors`] for traversal;
//! - [`Cursor`] - traverses and inspects the [`tree`];
//! - [`Node`] - stores a [`value`], a [`link`] to its parent and links to its
//!   currently populated children;
//! - [`Populate`] - a function used to create a [`node's`] children when a
//!   [`cursor`] visits a [`node`] that is not currently visited by another
//!   [`cursor`].
//!
//! ## Generic parameters
//!
//! Many types in this crate use the following generic parameters:
//!
//! - `Value` - the type of the [`value`] stored in each [`node`] through
//!   `Cow<'source, Value>`;
//! - `Source` - the element type of the `source` collection (`Vec<Source>`
//!   or `&[Source]`) used to populate [`nodes`]. The `source` collection is
//!   stored in each [`node`] through `Cow<'source, [Source]>`;
//! - `Error` - the `error` type returned by the [`population callback`].
//!
//! ## Safety
//!
//! A [`tree`] can be shared between threads, and multiple [`cursors`] may
//! traverse the same [`tree`] concurrently. Access to the `root` and to each
//! [`node's`] traversal state is synchronized internally. Each [`cursor`] still
//! borrows the [`tree`] for its entire lifetime and therefore cannot outlive
//! the [`tree`] from which it was created.
//!
//! ## Example
//!
//! The following example constructs a lazily populated binary [`tree`] from
//! serialized data represented as
//! `[ROOT, LEFT, RIGHT, LEFT-LEFT, LEFT-RIGHT, RIGHT-LEFT, RIGHT-RIGHT, ...]`:
//!
//! ```plain
//!                         ROOT                     
//!              ┌───────────┴───────────┐           
//!             LEFT                   RIGHT         
//!        ┌─────┴─────┐           ┌─────┴─────┐     
//!    LEFT-LEFT   LEFT-RIGHT RIGHT-LEFT  RIGHT-RIGHT
//!       ...         ...         ...         ...
//! ```
//!
//! This example uses `&str` as the [`node`] [`value`] type and `Option<&str>` as
//! the element type of the `source` collection. An absent value is represented
//! by `None`, which allows the `source` collection to describe missing nodes.
//!
//! The [`population callback`] creates a [`node's`] children from two
//! consecutive elements of the `source` collection, starting at the index
//! specified by the [`node's`] `range`. A [`node`] without a `range` is treated
//! as a leaf.
//!
//! ```rust
//! use std::{borrow::Cow, ops::Range};
//! use pretty_assertions::assert_eq;
//!
//! use lazy_tree::{
//!   Tree,
//!   node::{Builder, Link},
//!   cursor::{Direction, Target}
//! };
//!
//! // The type of the value stored in each node
//! type MyValue = str;
//!
//! // The type of an element in the source collection
//! type MySource<'source> = Option<&'source str>;
//!
//! // The error returned by the population callback
//! #[derive(Debug)]
//! enum MyError {
//!     // Indicates that the source collection is empty
//!     SourceIsEmpty,
//! };
//!
//! // A type alias for `Link` with the type parameters specified
//! type MyLink<'source> = Link<'source, MyValue, MySource<'source>, MyError>;
//!
//! // A type alias for a boxed slice of `MyLink`
//! type Children<'source> = Box<[MyLink<'source>]>;
//!
//! fn populate<'source>(
//!     source: impl Into<Cow<'source, [MySource<'source>]>>,
//!     range: Option<Range<usize>>,
//!     parent: MyLink<'source>,
//! ) -> Result<Children<'source>, MyError> {
//!     let range = if let Some(range) = range {
//!         range
//!     } else {
//!         // A node without a range is a leaf
//!         return Ok(Box::default());
//!     };
//!
//!     let source = source.into();
//!
//!     if source.is_empty() {
//!         return Err(MyError::SourceIsEmpty);
//!     }
//!
//!     // The range identifies the positions of the node's children in the
//!     // source collection
//!     let left_child = source.get(range.start).cloned();
//!     let right_child = source.get(range.start + 1).cloned();
//!
//!     let mut children = vec![];
//!
//!     if let Some(Some(value)) = left_child {
//!         children.push(
//!             Builder::new(
//!                 Cow::Borrowed(value),
//!                 source.clone(),
//!                 populate, // Reuses the same callback for child nodes
//!             )
//!             .with_parent(parent) // Required for upward traversal
//!             .with_range({
//!                 let start = range.start * 2 + 2;
//!                 if source.len() < start {
//!                     None
//!                 } else {
//!                     Some(start..source.len())
//!                 }
//!             })
//!             .build()
//!         );
//!     }
//!
//!     if let Some(Some(value)) = right_child {
//!         children.push(
//!             Builder::new(
//!                 Cow::Borrowed(value),
//!                 source.clone(),
//!                 populate, // Reuses the same callback for child nodes
//!             )
//!             .with_parent(parent) // Required for upward traversal
//!             .with_range({
//!                 let start = (range.start + 1) * 2 + 2;
//!                 if source.len() < start {
//!                     None
//!                 } else {
//!                     Some(start..source.len())
//!                 }
//!             })
//!             .build()
//!         );
//!     }
//!
//!     Ok(children.into_boxed_slice())
//! }
//!
//! //                                    
//! //                    ROOT            
//! //            ┌────────┴────────┐     
//! //           LEFT             RIGHT   
//! //       ┌────┴────┐            │     
//! //   LEFT-LEFT LEFT-RIGHT  RIGHT-RIGHT
//! //       │                            
//! // LEFT-LEFT-LEFT                     
//! //                                    
//! let source: &[Option<&str>] = &[
//!     Some("LEFT"),
//!     Some("RIGHT"),
//!     Some("LEFT-LEFT"),
//!     Some("LEFT-RIGHT"),
//!     None, // RIGHT-LEFT
//!     Some("RIGHT-RIGHT"),
//!     Some("LEFT-LEFT-LEFT"),
//!     None, // LEFT-LEFT-RIGHT
//! ];
//!
//! let tree = Tree::new(
//!     Builder::new("ROOT", source.clone(), populate)
//!         .with_range(0..source.len())
//!         .build()
//! );
//!
//! let mut cursor = tree.cursor().unwrap();
//! assert_eq!(cursor.path(), &[0]);
//! assert_eq!(cursor.value(), "ROOT");
//!
//! cursor.walk(Direction::Down(Target::First));
//! assert_eq!(cursor.path(), &[0, 0]);
//! assert_eq!(cursor.value(), "LEFT");
//!
//! cursor.walk(Direction::Right);
//! assert_eq!(cursor.path(), &[0, 1]);
//! assert_eq!(cursor.value(), "RIGHT");
//!
//! cursor.walk(Direction::Down(Target::First));
//! assert_eq!(cursor.path(), &[0, 1, 0]);
//! assert_eq!(cursor.value(), "RIGHT-RIGHT");
//! ```
//!
//! [Here] you could find a more complex example of a JSON parser build on top
//! of this crate.
//!
//! ## Features
//!
//! [`tree`]: Tree
//! [`tree's`]: Tree
//! [`trees`]: Tree
//! [`node`]: Node
//! [`nodes`]: Node
//! [`node's`]: Node
//! [`link`]: crate::node::Link
//! [`cursor`]: Cursor
//! [`cursor-based`]: Cursor
//! [`cursors`]: Cursor
//! [`callback`]: crate::node::Populate
//! [`population callback`]: crate::node::Populate
//! [`value`]: Node::value()
//! [Here]: https://github.com/mnmun/json
//!
#![doc = document_features::document_features!()]
//!
//! ## License
//!
//! [MIT](https://github.com/mnmun/lazy_tree/blob/main/LICENSE)

#![allow(dead_code)]

pub mod cursor;
pub mod node;
pub mod tree;

pub use cursor::Cursor;
pub use node::Node;
pub use tree::Tree;

#[cfg(test)]
mod tests {

    use std::{borrow::Cow, ops::Range, thread};

    use pretty_assertions::assert_eq;
    use serial_test::serial;

    use crate::{
        Cursor, Node, Tree,
        cursor::{Direction, Target},
        node::{Builder, Link},
    };

    type MyValue = str;
    type MySource<'source> = Option<&'source str>;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum MyError {
        SourceIsEmpty,
        RootIsNone,
    }

    type MyLink<'source> = Link<'source, MyValue, MySource<'source>, MyError>;
    type Children<'source> = Box<[MyLink<'source>]>;

    fn binary_tree<'source>(
        source: &'source [MySource<'source>],
    ) -> Tree<'source, MyValue, MySource<'source>, MyError> {
        fn populate<'source>(
            source: impl Into<Cow<'source, [MySource<'source>]>>,
            range: Option<Range<usize>>,
            parent: Link<'source, MyValue, MySource<'source>, MyError>,
        ) -> Result<Children<'source>, MyError> {
            let range = if let Some(range) = range {
                range
            } else {
                return Ok(Box::default());
            };

            let source = source.into();

            let left_child = source.get(range.start).cloned();
            let right_child = source.get(range.start + 1).cloned();

            let mut children = vec![];

            if let Some(Some(value)) = left_child {
                children.push(
                    Builder::new(
                        Cow::Borrowed(value),
                        source.clone(),
                        populate,
                    )
                    .with_parent(parent)
                    .with_range({
                        let start = range.start * 2 + 2;
                        if source.len() < start {
                            None
                        } else {
                            Some(start..source.len())
                        }
                    })
                    .build(),
                );
            }

            if let Some(Some(value)) = right_child {
                children.push(
                    Builder::new(
                        Cow::Borrowed(value),
                        source.clone(),
                        populate,
                    )
                    .with_parent(parent)
                    .with_range({
                        let start = (range.start + 1) * 2 + 2;
                        if source.len() < start {
                            None
                        } else {
                            Some(start..source.len())
                        }
                    })
                    .build(),
                );
            }

            Ok(children.into_boxed_slice())
        }

        Tree::new(
            Builder::new("ROOT", source, populate)
                .with_range(0..source.len())
                .build(),
        )
    }

    /// ```plain
    ///      ROOT
    ///   ┌───┴───┐
    ///   L       R
    /// ┌─┴─┐   ┌─┴─┐
    /// LL  LR  RL  RR
    ///     │   │
    ///    LRL RLL
    /// ```
    const BINARY_0: &[Option<&str>] = &[
        /* [0, 0]       */ Some("L"),
        /* [0, 1]       */ Some("R"),
        /* [0, 0, 0]    */ Some("LL"),
        /* [0, 0, 1]    */ Some("LR"),
        /* [0, 1, 0]    */ Some("RL"),
        /* [0, 1, 1]    */ Some("RR"),
        /* [0, 0, 0, 0] */ None, // LLL
        /* [0, 0, 0, 1] */ None, // LLR
        /* [0, 0, 1, 0] */ Some("LRL"),
        /* [0, 0, 1, 1] */ None, // LRR
        /* [0, 1, 0, 0] */ Some("RLL"),
        /* [0, 1, 0, 1] */ None, // RLR
        /* [0, 1, 1, 0] */ None, // RRL
        /* [0, 1, 1, 1] */ None, // RRR
    ];

    fn binary_0_assert_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(Node::from_link(cursor.link).value(), "ROOT");
        assert_eq!(*cursor.children(), ["L", "R"]);
    }

    fn binary_0_assert_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(Node::from_link(cursor.link).value(), "L");
        assert_eq!(*cursor.children(), ["LL", "LR"]);
    }

    fn binary_0_assert_0_1<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 1]);
        assert_eq!(Node::from_link(cursor.link).value(), "R");
        assert_eq!(*cursor.children(), ["RL", "RR",]);
    }

    fn binary_0_assert_0_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value(), "LL");
        assert_eq!(*cursor.children(), Vec::<&str>::default());
    }

    fn binary_0_assert_0_0_1<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 1]);
        assert_eq!(Node::from_link(cursor.link).value(), "LR");
        assert_eq!(*cursor.children(), ["LRL"]);
    }

    fn binary_0_assert_0_1_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 1, 0]);
        assert_eq!(Node::from_link(cursor.link).value(), "RL");
        assert_eq!(*cursor.children(), ["RLL" /*, "RLR" */]);
    }

    fn binary_0_assert_0_1_1<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 1, 1]);
        assert_eq!(Node::from_link(cursor.link).value(), "RR");
        assert_eq!(*cursor.children(), Vec::<&str>::default());
    }

    fn binary_0_assert_0_0_1_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 1, 0]);
        assert_eq!(Node::from_link(cursor.link).value(), "LRL");
        assert_eq!(*cursor.children(), Vec::<&str>::default());
    }

    fn binary_0_assert_0_1_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 1, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value(), "RLL");
        assert_eq!(*cursor.children(), Vec::<&str>::default());
    }

    /// ```plain
    ///  ROOT
    ///   │
    ///   L
    /// ┌─┴─┐
    /// LL  LR
    ///     │
    ///    LRL
    /// ```
    const BINARY_1: &[Option<&str>] = &[
        /* [0, 0]       */ Some("L"),
        /* [0, 1]       */ None, // R
        /* [0, 0, 0]    */ Some("LL"),
        /* [0, 0, 1]    */ Some("LR"),
        /* [0, 1, 0]    */ None, // RL
        /* [0, 1, 1]    */ None, // RR
        /* [0, 0, 0, 0] */ None, // LLL
        /* [0, 0, 0, 1] */ None, // LLR
        /* [0, 0, 1, 0] */ Some("LRL"),
        /* [0, 0, 1, 1] */ None, // LRR
        /* [0, 1, 0, 0] */ None, // RLL
        /* [0, 1, 0, 1] */ None, // RLR
        /* [0, 1, 1, 0] */ None, // RRL
        /* [0, 1, 1, 1] */ None, // RRR
    ];

    fn binary_1_assert_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(Node::from_link(cursor.link).value(), "ROOT");
        assert_eq!(*cursor.children(), ["L"]);
    }

    fn binary_1_assert_0_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value(), "LL");
        assert_eq!(*cursor.children(), Vec::<&str>::default());
    }

    fn binary_1_assert_0_0_1<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 1]);
        assert_eq!(Node::from_link(cursor.link).value(), "LR");
        assert_eq!(*cursor.children(), ["LRL"]);
    }

    fn binary_1_assert_0_0_1_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 1, 0]);
        assert_eq!(Node::from_link(cursor.link).value(), "LRL");
        assert_eq!(*cursor.children(), Vec::<&str>::default());
    }

    /// ```plain
    ///   ROOT
    ///    │
    ///    L
    ///  ┌─┴─┐
    ///  LL  LR
    ///  │
    /// LLL
    /// ```
    const BINARY_2: &[Option<&str>] = &[
        /* [0, 0]       */ Some("L"),
        /* [0, 1]       */ None, // R
        /* [0, 0, 0]    */ Some("LL"),
        /* [0, 0, 1]    */ Some("LR"),
        /* [0, 1, 0]    */ None, // RL
        /* [0, 1, 1]    */ None, // RR
        /* [0, 0, 0, 0] */ Some("LLL"),
        /* [0, 0, 0, 1] */ None, // LLR
        /* [0, 0, 1, 0] */ None, // LRL
        /* [0, 0, 1, 1] */ None, // LRR
        /* [0, 1, 0, 0] */ None, // RLL
        /* [0, 1, 0, 1] */ None, // RLR
        /* [0, 1, 1, 0] */ None, // RRL
        /* [0, 1, 1, 1] */ None, // RRR
    ];

    fn binary_2_assert_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(Node::from_link(cursor.link).value(), "ROOT");
        assert_eq!(*cursor.children(), ["L"]);
    }

    fn binary_2_assert_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(Node::from_link(cursor.link).value(), "L");
        assert_eq!(*cursor.children(), ["LL", "LR"]);
    }

    fn binary_2_assert_0_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value(), "LL");
        assert_eq!(*cursor.children(), ["LLL"]);
    }

    fn binary_2_assert_0_0_1<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 1]);
        assert_eq!(Node::from_link(cursor.link).value(), "LR");
        assert_eq!(*cursor.children(), Vec::<&str>::default());
    }

    fn binary_2_assert_0_0_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value(), "LLL");
        assert_eq!(*cursor.children(), Vec::<&str>::default());
    }

    /// ```plain
    ///   ROOT
    ///    │
    ///    L
    ///  ┌─┴─┐
    ///  LL  LR
    ///  │   │
    /// LLL LRL
    ///  │
    /// LLLL
    /// ```
    const BINARY_3: &[Option<&str>] = &[
        /* [0, 0]          */ Some("L"),
        /* [0, 1]          */ None, // R
        /* [0, 0, 0]       */ Some("LL"),
        /* [0, 0, 1]       */ Some("LR"),
        /* [0, 1, 0]       */ None, // RL
        /* [0, 1, 1]       */ None, // RR
        /* [0, 0, 0, 0]    */ Some("LLL"),
        /* [0, 0, 0, 1]    */ None, // LLR
        /* [0, 0, 1, 0]    */ Some("LRL"),
        /* [0, 0, 1, 1]    */ None, // LRR
        /* [0, 1, 0, 0]    */ None, // RLL
        /* [0, 1, 0, 1]    */ None, // RLR
        /* [0, 1, 1, 0]    */ None, // RRL
        /* [0, 1, 1, 1]    */ None, // RRR
        /* [0, 0, 0, 0, 0] */ Some("LLLL"),
        /* [0, 0, 0, 0, 1] */ None, // LLLR
    ];

    fn binary_3_assert_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(Node::from_link(cursor.link).value(), "ROOT");
        assert_eq!(*cursor.children(), ["L"]);
    }

    fn binary_3_assert_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(Node::from_link(cursor.link).value(), "L");
        assert_eq!(*cursor.children(), ["LL", "LR"]);
    }

    fn binary_3_assert_0_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value(), "LL");
        assert_eq!(*cursor.children(), ["LLL"]);
    }

    fn binary_3_assert_0_0_1<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 1]);
        assert_eq!(Node::from_link(cursor.link).value(), "LR");
        assert_eq!(*cursor.children(), ["LRL"]);
    }

    fn binary_3_assert_0_0_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value(), "LLL");
        assert_eq!(*cursor.children(), ["LLLL"]);
    }

    fn binary_3_assert_0_0_1_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 1, 0]);
        assert_eq!(Node::from_link(cursor.link).value(), "LRL");
        assert_eq!(*cursor.children(), Vec::<&str>::default());
    }

    fn binary_3_assert_0_0_0_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 0, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value(), "LLLL");
        assert_eq!(*cursor.children(), Vec::<&str>::default());
    }

    /// ```plain
    ///   ROOT
    ///    │
    ///    L
    ///  ┌─┴─┐
    ///  LL  LR
    ///  │   │
    /// LLL LRL
    ///      │
    ///     LRLL
    /// ```
    const BINARY_4: &[Option<&str>] = &[
        /* [0, 0]          */ Some("L"),
        /* [0, 1]          */ None, // R
        /* [0, 0, 0]       */ Some("LL"),
        /* [0, 0, 1]       */ Some("LR"),
        /* [0, 1, 0]       */ None, // RL
        /* [0, 1, 1]       */ None, // RR
        /* [0, 0, 0, 0]    */ Some("LLL"),
        /* [0, 0, 0, 1]    */ None, // LLR
        /* [0, 0, 1, 0]    */ Some("LRL"),
        /* [0, 0, 1, 1]    */ None, // LRR
        /* [0, 1, 0, 0]    */ None, // RLL
        /* [0, 1, 0, 1]    */ None, // RLR
        /* [0, 1, 1, 0]    */ None, // RRL
        /* [0, 1, 1, 1]    */ None, // RRR
        /* [0, 0, 0, 0, 0] */ None, // LLLL
        /* [0, 0, 0, 0, 1] */ None, // LLLR
        /* [0, 0, 0, 1, 0] */ None, // LLRL
        /* [0, 0, 0, 1, 1] */ None, // LLRR
        /* [0, 0, 1, 0, 0] */ Some("LRLL"),
        /* [0, 0, 1, 0, 1] */ None, // LRLR
    ];

    fn binary_4_assert_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(Node::from_link(cursor.link).value(), "ROOT");
        assert_eq!(*cursor.children(), ["L"]);
    }

    fn binary_4_assert_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(Node::from_link(cursor.link).value(), "L");
        assert_eq!(*cursor.children(), ["LL", "LR"]);
    }

    fn binary_4_assert_0_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value(), "LL");
        assert_eq!(*cursor.children(), ["LLL"]);
    }

    fn binary_4_assert_0_0_1<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 1]);
        assert_eq!(Node::from_link(cursor.link).value(), "LR");
        assert_eq!(*cursor.children(), ["LRL"]);
    }

    fn binary_4_assert_0_0_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value(), "LLL");
        assert_eq!(*cursor.children(), Vec::<&str>::default());
    }

    fn binary_4_assert_0_0_1_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 1, 0]);
        assert_eq!(Node::from_link(cursor.link).value(), "LRL");
        assert_eq!(*cursor.children(), ["LRLL"]);
    }

    fn binary_4_assert_0_0_1_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 1, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value(), "LRLL");
        assert_eq!(*cursor.children(), Vec::<&str>::default());
    }

    /// ```plain
    /// ROOT
    ///  │
    ///  L
    ///  │
    ///  LL
    ///  │
    /// LLL
    ///  │
    /// LLLL
    /// ```
    const LIST: &[Option<&str>] = &[
        /* [0, 0]          */ Some("L"),
        /* [0, 1]          */ None, // R
        /* [0, 0, 0]       */ Some("LL"),
        /* [0, 0, 1]       */ None, // LR
        /* [0, 1, 0]       */ None, // RL
        /* [0, 1, 1]       */ None, // RR
        /* [0, 0, 0, 0]    */ Some("LLL"),
        /* [0, 0, 0, 1]    */ None, // LLR
        /* [0, 0, 1, 0]    */ None, // LRL
        /* [0, 0, 1, 1]    */ None, // LRR
        /* [0, 1, 0, 0]    */ None, // RLL
        /* [0, 1, 0, 1]    */ None, // RLR
        /* [0, 1, 1, 0]    */ None, // RRL
        /* [0, 1, 1, 1]    */ None, // RRR
        /* [0, 0, 0, 0, 0] */ Some("LLLL"),
        /* [0, 0, 0, 0, 1] */ None, // LLLR
    ];

    fn list_assert_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(Node::from_link(cursor.link).value(), "ROOT");
        assert_eq!(*cursor.children(), ["L"]);
    }

    fn list_assert_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(Node::from_link(cursor.link).value(), "L");
        assert_eq!(*cursor.children(), ["LL"]);
    }

    fn list_assert_0_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value(), "LL");
        assert_eq!(*cursor.children(), ["LLL"]);
    }

    fn list_assert_0_0_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value(), "LLL");
        assert_eq!(*cursor.children(), ["LLLL"]);
    }

    fn list_assert_0_0_0_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 0, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value(), "LLLL");
        assert_eq!(*cursor.children(), Vec::<&str>::default());
    }

    /// ```plain
    /// ROOT
    /// ```
    const EMPTY: &[Option<&str>] = &[None, None];

    fn empty_assert_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(Node::from_link(cursor.link).value(), "ROOT");
        assert_eq!(*cursor.children(), Vec::<&str>::default());
    }

    /// ```plain
    ///               ROOT
    ///        ┌───────┴───────┐
    ///        L               R
    ///    ┌───┴───┐       ┌───┴───┐
    ///    LL      LR      RL      RR
    ///  ┌─┴─┐   ┌─┴─┐   ┌─┴─┐   ┌─┴─┐
    /// LLL LLR LRL LRR RLL RLR RRL RRR
    /// ```
    const BINARY_FULL: &[Option<&str>] = &[
        /* [0, 0]       */ Some("L"),
        /* [0, 1]       */ Some("R"),
        /* [0, 0, 0]    */ Some("LL"),
        /* [0, 0, 1]    */ Some("LR"),
        /* [0, 1, 0]    */ Some("RL"),
        /* [0, 1, 1]    */ Some("RR"),
        /* [0, 0, 0, 0] */ Some("LLL"),
        /* [0, 0, 0, 1] */ Some("LLR"),
        /* [0, 0, 1, 0] */ Some("LRL"),
        /* [0, 0, 1, 1] */ Some("LRR"),
        /* [0, 1, 0, 0] */ Some("RLL"),
        /* [0, 1, 0, 1] */ Some("RLR"),
        /* [0, 1, 1, 0] */ Some("RRL"),
        /* [0, 1, 1, 1] */ Some("RRR"),
    ];

    fn binary_full_assert_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(Node::from_link(cursor.link).value(), "ROOT");
        assert_eq!(*cursor.children(), ["L", "R"]);
    }

    fn binary_full_assert_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(Node::from_link(cursor.link).value(), "L");
        assert_eq!(*cursor.children(), ["LL", "LR"]);
    }

    fn binary_full_assert_0_1<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 1]);
        assert_eq!(Node::from_link(cursor.link).value(), "R");
        assert_eq!(*cursor.children(), ["RL", "RR",]);
    }

    fn binary_full_assert_0_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value(), "LL");
        assert_eq!(*cursor.children(), ["LLL", "LLR"]);
    }

    fn binary_full_assert_0_0_1<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 1]);
        assert_eq!(Node::from_link(cursor.link).value(), "LR");
        assert_eq!(*cursor.children(), ["LRL", "LRR"]);
    }

    fn binary_full_assert_0_1_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 1, 0]);
        assert_eq!(Node::from_link(cursor.link).value(), "RL");
        assert_eq!(*cursor.children(), ["RLL", "RLR"]);
    }

    fn binary_full_assert_0_1_1<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 1, 1]);
        assert_eq!(Node::from_link(cursor.link).value(), "RR");
        assert_eq!(*cursor.children(), ["RRL", "RRR"]);
    }

    fn binary_full_assert_0_0_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value(), "LLL");
        assert_eq!(*cursor.children(), Vec::<&str>::default());
    }

    fn binary_full_assert_0_0_0_1<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 0, 1]);
        assert_eq!(Node::from_link(cursor.link).value(), "LLR");
        assert_eq!(*cursor.children(), Vec::<&str>::default());
    }

    fn binary_full_assert_0_0_1_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 1, 0]);
        assert_eq!(Node::from_link(cursor.link).value(), "LRL");
        assert_eq!(*cursor.children(), Vec::<&str>::default());
    }

    fn binary_full_assert_0_0_1_1<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 1, 1]);
        assert_eq!(Node::from_link(cursor.link).value(), "LRR");
        assert_eq!(*cursor.children(), Vec::<&str>::default());
    }

    fn binary_full_assert_0_1_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 1, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value(), "RLL");
        assert_eq!(*cursor.children(), Vec::<&str>::default());
    }

    fn binary_full_assert_0_1_0_1<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 1, 0, 1]);
        assert_eq!(Node::from_link(cursor.link).value(), "RLR");
        assert_eq!(*cursor.children(), Vec::<&str>::default());
    }

    fn binary_full_assert_0_1_1_0<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 1, 1, 0]);
        assert_eq!(Node::from_link(cursor.link).value(), "RRL");
        assert_eq!(*cursor.children(), Vec::<&str>::default());
    }

    fn binary_full_assert_0_1_1_1<'tree, 'source>(
        cursor: &Cursor<'tree, 'source, str, Option<&str>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 1, 1, 1]);
        assert_eq!(Node::from_link(cursor.link).value(), "RRR");
        assert_eq!(*cursor.children(), Vec::<&str>::default());
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    fn go_to() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        binary_0_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0]).unwrap());
        binary_0_assert_0_0(&cursor);

        assert!(cursor.go_to(&[0, 1]).unwrap());
        binary_0_assert_0_1(&cursor);

        assert!(cursor.go_to(&[0, 0, 0]).unwrap());
        binary_0_assert_0_0_0(&cursor);

        assert!(cursor.go_to(&[0, 0, 1]).unwrap());
        binary_0_assert_0_0_1(&cursor);

        assert!(cursor.go_to(&[0, 1, 0]).unwrap());
        binary_0_assert_0_1_0(&cursor);

        assert!(cursor.go_to(&[0, 1, 1]).unwrap());
        binary_0_assert_0_1_1(&cursor);

        assert!(cursor.go_to(&[0, 0, 1, 0]).unwrap());
        binary_0_assert_0_0_1_0(&cursor);

        assert!(cursor.go_to(&[0, 1, 0, 0]).unwrap());
        binary_0_assert_0_1_0_0(&cursor);

        assert!(!cursor.go_to(&[0, 0, 0, 0, 0, 0, 0]).unwrap());
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    fn wrong_go_to() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        binary_0_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0]).unwrap());
        binary_0_assert_0_0(&cursor);

        assert!(!cursor.go_to(&[0, 0, 2]).unwrap());
        binary_0_assert_0_0(&cursor);

        assert!(!cursor.go_to(&[0, 1, 2]).unwrap());
        binary_0_assert_0_1(&cursor);

        assert!(!cursor.go_to(&[0, 3, 0]).unwrap());
        binary_0_assert_0(&cursor);

        assert!(!cursor.go_to(&[3, 0, 0]).unwrap());
        binary_0_assert_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Jump from ROOT to LL
    fn jump_up_from_root() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        binary_0_assert_0(&cursor);

        assert!(cursor.jump(Direction::Up).unwrap());
        binary_0_assert_0_0_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Jump from LL to L
    fn jump_up() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        binary_0_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0, 0]).unwrap());
        binary_0_assert_0_0_0(&cursor);

        assert!(cursor.jump(Direction::Up).unwrap());
        binary_0_assert_0_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`EMPTY`]
    fn jump_up_from_root_in_empty_tree() {
        let tree = binary_tree(EMPTY);
        let mut cursor = tree.cursor().unwrap();
        empty_assert_0(&cursor);

        assert!(cursor.jump(Direction::Up).unwrap());
        empty_assert_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Jump from RLL to ROOT
    fn jump_down_from_leaf() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        binary_0_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 1, 0, 0]).unwrap());
        binary_0_assert_0_1_0_0(&cursor);
        assert!(cursor.jump(Direction::Down(Target::First)).unwrap());
        binary_0_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 1, 0, 0]).unwrap());
        binary_0_assert_0_1_0_0(&cursor);
        assert!(cursor.jump(Direction::Down(Target::Last)).unwrap());
        binary_0_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 1, 0, 0]).unwrap());
        binary_0_assert_0_1_0_0(&cursor);
        assert!(cursor.jump(Direction::Down(Target::Index(0))).unwrap());
        binary_0_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 1, 0, 0]).unwrap());
        binary_0_assert_0_1_0_0(&cursor);
        assert!(cursor.jump(Direction::Down(Target::Index(2))).unwrap());
        binary_0_assert_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Jumps from:
    /// - L to LL
    /// - L to LR
    /// - L to ROOT
    fn jump_down() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        binary_0_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0]).unwrap());
        binary_0_assert_0_0(&cursor);
        assert!(cursor.jump(Direction::Down(Target::First)).unwrap());
        binary_0_assert_0_0_0(&cursor);

        assert!(cursor.go_to(&[0, 0]).unwrap());
        binary_0_assert_0_0(&cursor);
        assert!(cursor.jump(Direction::Down(Target::Last)).unwrap());
        binary_0_assert_0_0_1(&cursor);

        assert!(cursor.go_to(&[0, 0]).unwrap());
        binary_0_assert_0_0(&cursor);
        assert!(cursor.jump(Direction::Down(Target::Index(1))).unwrap());
        binary_0_assert_0_0_1(&cursor);

        assert!(cursor.go_to(&[0, 0]).unwrap());
        binary_0_assert_0_0(&cursor);
        assert!(cursor.jump(Direction::Down(Target::Index(2))).unwrap());
        binary_0_assert_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`EMPTY`]
    fn jump_down_from_root_in_empty_tree() {
        let tree = binary_tree(EMPTY);
        let mut cursor = tree.cursor().unwrap();
        empty_assert_0(&cursor);

        assert!(cursor.jump(Direction::Down(Target::First)).unwrap());
        empty_assert_0(&cursor);

        assert!(cursor.jump(Direction::Down(Target::Last)).unwrap());
        empty_assert_0(&cursor);

        assert!(cursor.jump(Direction::Down(Target::Index(0))).unwrap());
        empty_assert_0(&cursor);

        assert!(cursor.jump(Direction::Down(Target::Index(2))).unwrap());
        empty_assert_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Attempt to walk from ROOT to LL
    fn walk_up_from_root() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        binary_0_assert_0(&cursor);

        assert!(!cursor.walk(Direction::Up).unwrap());
        binary_0_assert_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Move from LL to L
    fn walk_up() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        binary_0_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0, 0]).unwrap());
        binary_0_assert_0_0_0(&cursor);

        assert!(cursor.walk(Direction::Up).unwrap());
        binary_0_assert_0_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`EMPTY`]
    fn walk_up_from_root_in_empty_tree() {
        let tree = binary_tree(EMPTY);
        let mut cursor = tree.cursor().unwrap();
        empty_assert_0(&cursor);

        assert!(!cursor.walk(Direction::Up).unwrap());
        empty_assert_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Attempt to walk from RLL to ROOT
    fn walk_down_from_leaf() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        binary_0_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 1, 0, 0]).unwrap());
        binary_0_assert_0_1_0_0(&cursor);

        assert!(!cursor.walk(Direction::Down(Target::First)).unwrap());
        binary_0_assert_0_1_0_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Attempts to walk from:
    /// - L to LL
    /// - L to LR
    /// - L to ROOT
    fn walk_down() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        binary_0_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0]).unwrap());
        binary_0_assert_0_0(&cursor);
        assert!(cursor.walk(Direction::Down(Target::First)).unwrap());
        binary_0_assert_0_0_0(&cursor);

        assert!(cursor.go_to(&[0, 0]).unwrap());
        binary_0_assert_0_0(&cursor);
        assert!(cursor.walk(Direction::Down(Target::Last)).unwrap());
        binary_0_assert_0_0_1(&cursor);

        assert!(cursor.go_to(&[0, 0]).unwrap());
        binary_0_assert_0_0(&cursor);
        assert!(cursor.walk(Direction::Down(Target::Index(1))).unwrap());
        binary_0_assert_0_0_1(&cursor);

        assert!(cursor.go_to(&[0, 0]).unwrap());
        binary_0_assert_0_0(&cursor);
        assert!(!cursor.walk(Direction::Down(Target::Index(2))).unwrap());
        binary_0_assert_0_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`EMPTY`]
    fn walk_down_from_root_in_empty_tree() {
        let tree = binary_tree(EMPTY);
        let mut cursor = tree.cursor().unwrap();
        empty_assert_0(&cursor);

        assert!(!cursor.walk(Direction::Down(Target::First)).unwrap());
        empty_assert_0(&cursor);

        assert!(!cursor.walk(Direction::Down(Target::Last)).unwrap());
        empty_assert_0(&cursor);

        assert!(!cursor.walk(Direction::Down(Target::Index(0))).unwrap());
        empty_assert_0(&cursor);

        assert!(!cursor.walk(Direction::Down(Target::Index(2))).unwrap());
        empty_assert_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Jump from LL to RR
    fn jump_left_across_tree_different_branch_same_depth() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        binary_0_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0, 0]).unwrap());
        binary_0_assert_0_0_0(&cursor);

        assert!(cursor.jump(Direction::Left).unwrap());
        binary_0_assert_0_1_1(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Jump from L to R
    fn jump_left_across_tree_same_branch_same_depth() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        binary_0_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0]).unwrap());
        binary_0_assert_0_0(&cursor);

        assert!(cursor.jump(Direction::Left).unwrap());
        binary_0_assert_0_1(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_2`]
    ///
    /// Jump from LLL to LR
    fn jump_left_across_tree_to_shorter_branch() {
        let tree = binary_tree(BINARY_2);
        let mut cursor = tree.cursor().unwrap();
        binary_2_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0, 0, 0]).unwrap());
        binary_2_assert_0_0_0_0(&cursor);

        assert!(cursor.jump(Direction::Left).unwrap());
        binary_2_assert_0_0_1(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_1`]
    ///
    /// Jump from LL to LR
    fn jump_left_across_tree_to_longer_branch() {
        let tree = binary_tree(BINARY_1);
        let mut cursor = tree.cursor().unwrap();

        binary_1_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0, 0]).unwrap());
        binary_1_assert_0_0_0(&cursor);

        assert!(cursor.jump(Direction::Left).unwrap());
        binary_1_assert_0_0_1(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Jump from R to L
    fn jump_left_in_branch_same_depth() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        binary_0_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 1]).unwrap());
        binary_0_assert_0_1(&cursor);

        assert!(cursor.jump(Direction::Left).unwrap());
        binary_0_assert_0_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_1`]
    ///
    /// Jump from LRL to LL
    fn jump_left_between_branches_to_shorter_branch() {
        let tree = binary_tree(BINARY_1);
        let mut cursor = tree.cursor().unwrap();
        binary_1_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0, 1, 0]).unwrap());
        binary_1_assert_0_0_1_0(&cursor);

        assert!(cursor.jump(Direction::Left).unwrap());
        binary_1_assert_0_0_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_3`]
    ///
    /// Jump from LRL to LLL
    fn jump_left_between_branches_to_longer_branch() {
        let tree = binary_tree(BINARY_3);
        let mut cursor = tree.cursor().unwrap();
        binary_3_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0, 1, 0]).unwrap());
        binary_3_assert_0_0_1_0(&cursor);

        assert!(cursor.jump(Direction::Left).unwrap());
        binary_3_assert_0_0_0_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Jump from RL to LR
    fn jump_left_between_branches_same_depth() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        binary_0_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 1, 0]).unwrap());
        binary_0_assert_0_1_0(&cursor);

        assert!(cursor.jump(Direction::Left).unwrap());
        binary_0_assert_0_0_1(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`LIST`]
    fn jump_left_in_list() {
        let tree = binary_tree(LIST);
        let mut cursor = tree.cursor().unwrap();
        list_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0, 0]).unwrap());
        list_assert_0_0_0(&cursor);

        assert!(cursor.jump(Direction::Left).unwrap());
        list_assert_0_0_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`EMPTY`]
    fn jump_left_in_empty_tree() {
        let tree = binary_tree(EMPTY);
        let mut cursor = tree.cursor().unwrap();
        empty_assert_0(&cursor);

        assert!(cursor.jump(Direction::Left).unwrap());
        empty_assert_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Jump from RR to LL
    fn jump_right_across_tree_different_branch_same_depth() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        binary_0_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 1, 1]).unwrap());
        binary_0_assert_0_1_1(&cursor);

        assert!(cursor.jump(Direction::Right).unwrap());
        binary_0_assert_0_0_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Jump from R to L
    fn jump_right_across_tree_same_branch_same_depth() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        binary_0_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 1]).unwrap());
        binary_0_assert_0_1(&cursor);

        assert!(cursor.jump(Direction::Right).unwrap());
        binary_0_assert_0_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_1`]
    ///
    /// Jump from LRL to LL
    fn jump_right_across_tree_to_shorter_branch() {
        let tree = binary_tree(BINARY_1);
        let mut cursor = tree.cursor().unwrap();
        binary_1_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0, 1, 0]).unwrap());
        binary_1_assert_0_0_1_0(&cursor);

        assert!(cursor.jump(Direction::Right).unwrap());
        binary_1_assert_0_0_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_2`]
    ///
    /// Jump from LR to LL
    fn jump_right_across_tree_to_longer_branch() {
        let tree = binary_tree(BINARY_2);
        let mut cursor = tree.cursor().unwrap();
        binary_2_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0, 1]).unwrap());
        binary_2_assert_0_0_1(&cursor);

        assert!(cursor.jump(Direction::Right).unwrap());
        binary_2_assert_0_0_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Jump from L to R
    fn jump_right_in_branch_same_depth() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        binary_0_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0]).unwrap());
        binary_0_assert_0_0(&cursor);

        assert!(cursor.jump(Direction::Right).unwrap());
        binary_0_assert_0_1(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_2`]
    ///
    /// Jump from LLL to LR
    fn jump_right_between_branches_to_shorter_branch() {
        let tree = binary_tree(BINARY_2);
        let mut cursor = tree.cursor().unwrap();
        binary_2_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0, 0, 0]).unwrap());
        binary_2_assert_0_0_0_0(&cursor);

        assert!(cursor.jump(Direction::Right).unwrap());
        binary_2_assert_0_0_1(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_4`]
    ///
    /// Jump from LLL to LRL
    fn jump_right_between_branches_to_longer_branch() {
        let tree = binary_tree(BINARY_4);
        let mut cursor = tree.cursor().unwrap();
        binary_4_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0, 0, 0]).unwrap());
        binary_4_assert_0_0_0_0(&cursor);

        assert!(cursor.jump(Direction::Right).unwrap());
        binary_4_assert_0_0_1_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Jump from LR to RL
    fn jump_right_between_branches_same_depth() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        binary_0_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0, 1]).unwrap());
        binary_0_assert_0_0_1(&cursor);

        assert!(cursor.jump(Direction::Right).unwrap());
        binary_0_assert_0_1_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`LIST`]
    fn jump_right_in_list() {
        let tree = binary_tree(LIST);
        let mut cursor = tree.cursor().unwrap();
        list_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0, 0]).unwrap());
        list_assert_0_0_0(&cursor);

        assert!(cursor.jump(Direction::Right).unwrap());
        list_assert_0_0_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`EMPTY`]
    fn jump_right_in_empty_tree() {
        let tree = binary_tree(EMPTY);
        let mut cursor = tree.cursor().unwrap();
        empty_assert_0(&cursor);

        assert!(cursor.jump(Direction::Right).unwrap());
        empty_assert_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Attempt to walk from LL to RR
    fn walk_left_across_tree_different_branch_same_depth() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        binary_0_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0, 0]).unwrap());
        binary_0_assert_0_0_0(&cursor);

        assert!(!cursor.walk(Direction::Left).unwrap());
        binary_0_assert_0_0_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Attempt to walk from L to R
    fn walk_left_across_tree_same_branch_same_depth() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        binary_0_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0]).unwrap());
        binary_0_assert_0_0(&cursor);

        assert!(!cursor.walk(Direction::Left).unwrap());
        binary_0_assert_0_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_2`]
    ///
    /// Attempt to walk from LLL to LR
    fn walk_left_across_tree_to_shorter_branch() {
        let tree = binary_tree(BINARY_2);
        let mut cursor = tree.cursor().unwrap();
        binary_2_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0, 0, 0]).unwrap());
        binary_2_assert_0_0_0_0(&cursor);

        assert!(!cursor.walk(Direction::Left).unwrap());
        binary_2_assert_0_0_0_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_1`]
    ///
    /// Attempt to walk from LL to LR
    fn walk_left_across_tree_to_longer_branch() {
        let tree = binary_tree(BINARY_1);
        let mut cursor = tree.cursor().unwrap();

        binary_1_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0, 0]).unwrap());
        binary_1_assert_0_0_0(&cursor);

        assert!(!cursor.walk(Direction::Left).unwrap());
        binary_1_assert_0_0_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Move from R to L
    fn walk_left_in_branch_same_depth() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        binary_0_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 1]).unwrap());
        binary_0_assert_0_1(&cursor);

        assert!(cursor.walk(Direction::Left).unwrap());
        binary_0_assert_0_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_1`]
    ///
    /// Attempt to walk from LRL to LL
    fn walk_left_between_branches_to_shorter_branch() {
        let tree = binary_tree(BINARY_1);
        let mut cursor = tree.cursor().unwrap();
        binary_1_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0, 1, 0]).unwrap());
        binary_1_assert_0_0_1_0(&cursor);

        assert!(!cursor.walk(Direction::Left).unwrap());
        binary_1_assert_0_0_1_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_3`]
    ///
    /// Attempt to walk from LRL to LLL
    fn walk_left_between_branches_to_longer_branch() {
        let tree = binary_tree(BINARY_3);
        let mut cursor = tree.cursor().unwrap();
        binary_3_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0, 1, 0]).unwrap());
        binary_3_assert_0_0_1_0(&cursor);

        assert!(!cursor.walk(Direction::Left).unwrap());
        binary_3_assert_0_0_1_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Attempt to walk from RL to LR
    fn walk_left_between_branches_same_depth() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        binary_0_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 1, 0]).unwrap());
        binary_0_assert_0_1_0(&cursor);

        assert!(!cursor.walk(Direction::Left).unwrap());
        binary_0_assert_0_1_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`LIST`]
    fn walk_left_in_list() {
        let tree = binary_tree(LIST);
        let mut cursor = tree.cursor().unwrap();
        list_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0, 0]).unwrap());
        list_assert_0_0_0(&cursor);

        assert!(!cursor.walk(Direction::Left).unwrap());
        list_assert_0_0_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`EMPTY`]
    fn walk_left_in_empty_tree() {
        let tree = binary_tree(EMPTY);
        let mut cursor = tree.cursor().unwrap();
        empty_assert_0(&cursor);

        assert!(!cursor.walk(Direction::Left).unwrap());
        empty_assert_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Attempt to walk from RR to LL
    fn walk_right_across_tree_different_branch_same_depth() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        binary_0_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 1, 1]).unwrap());
        binary_0_assert_0_1_1(&cursor);

        assert!(!cursor.walk(Direction::Right).unwrap());
        binary_0_assert_0_1_1(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Attempt to walk from R to L
    fn walk_right_across_tree_same_branch_same_depth() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        binary_0_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 1]).unwrap());
        binary_0_assert_0_1(&cursor);

        assert!(!cursor.walk(Direction::Right).unwrap());
        binary_0_assert_0_1(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_1`]
    ///
    /// Attempt to walk from LRL to LL
    fn walk_right_across_tree_to_shorter_branch() {
        let tree = binary_tree(BINARY_1);
        let mut cursor = tree.cursor().unwrap();
        binary_1_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0, 1, 0]).unwrap());
        binary_1_assert_0_0_1_0(&cursor);

        assert!(!cursor.walk(Direction::Right).unwrap());
        binary_1_assert_0_0_1_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_2`]
    ///
    /// Attempt to walk from LR to LL
    fn walk_right_across_tree_to_longer_branch() {
        let tree = binary_tree(BINARY_2);
        let mut cursor = tree.cursor().unwrap();
        binary_2_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0, 1]).unwrap());
        binary_2_assert_0_0_1(&cursor);

        assert!(!cursor.walk(Direction::Right).unwrap());
        binary_2_assert_0_0_1(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Move from L to R
    fn walk_right_in_branch_same_depth() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        binary_0_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0]).unwrap());
        binary_0_assert_0_0(&cursor);

        assert!(cursor.walk(Direction::Right).unwrap());
        binary_0_assert_0_1(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_2`]
    ///
    /// Attempt to walk from LLL to LR
    fn walk_right_between_branches_to_shorter_branch() {
        let tree = binary_tree(BINARY_2);
        let mut cursor = tree.cursor().unwrap();
        binary_2_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0, 0, 0]).unwrap());
        binary_2_assert_0_0_0_0(&cursor);

        assert!(!cursor.walk(Direction::Right).unwrap());
        binary_2_assert_0_0_0_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_4`]
    ///
    /// Attempt to walk from LLL to LRL
    fn walk_right_between_branches_to_longer_branch() {
        let tree = binary_tree(BINARY_4);
        let mut cursor = tree.cursor().unwrap();
        binary_4_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0, 0, 0]).unwrap());
        binary_4_assert_0_0_0_0(&cursor);

        assert!(!cursor.walk(Direction::Right).unwrap());
        binary_4_assert_0_0_0_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Attempt to walk from LR to RL
    fn walk_right_between_branches_same_depth() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        binary_0_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0, 1]).unwrap());
        binary_0_assert_0_0_1(&cursor);

        assert!(!cursor.walk(Direction::Right).unwrap());
        binary_0_assert_0_0_1(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`LIST`]
    fn walk_right_in_list() {
        let tree = binary_tree(LIST);
        let mut cursor = tree.cursor().unwrap();
        list_assert_0(&cursor);

        assert!(cursor.go_to(&[0, 0, 0]).unwrap());
        list_assert_0_0_0(&cursor);

        assert!(!cursor.walk(Direction::Right).unwrap());
        list_assert_0_0_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`EMPTY`]
    fn walk_right_in_empty_tree() {
        let tree = binary_tree(EMPTY);
        let mut cursor = tree.cursor().unwrap();
        empty_assert_0(&cursor);

        assert!(!cursor.walk(Direction::Right).unwrap());
        empty_assert_0(&cursor);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_FULL`]
    fn cursor_drop() {
        let tree = binary_tree(BINARY_FULL);
        let mut cursor = tree.cursor().unwrap();

        binary_full_assert_0(&cursor);
        assert_eq!(*cursor.visitors.lock().unwrap(), 1);
        unsafe { assert!(!(&*cursor.children.get()).is_empty()) }
        unsafe {
            let root = Node::from_link(tree.root);
            assert_eq!(*root.visitors.lock().unwrap(), 1);
            assert!(!(&*root.children.get()).is_empty());
        };

        assert!(cursor.walk(Direction::Down(Target::First)).unwrap());
        binary_full_assert_0_0(&cursor);
        assert_eq!(*cursor.visitors.lock().unwrap(), 1);
        unsafe { assert!(!(&*cursor.children.get()).is_empty()) }
        unsafe {
            let root = Node::from_link(tree.root);
            assert_eq!(*root.visitors.lock().unwrap(), 1);
            assert!(!(&*root.children.get()).is_empty());
        };

        drop(cursor);
        unsafe {
            let root = Node::from_link(tree.root);
            assert_eq!(*root.visitors.lock().unwrap(), 0);
            assert!((&*root.children.get()).is_empty());
        };
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_FULL`]
    fn multiple_cursors_one_thread() {
        let tree = binary_tree(BINARY_FULL);

        let mut cursor_0 = tree.cursor().unwrap();
        binary_full_assert_0(&cursor_0);
        let mut cursor_1 = tree.cursor().unwrap();
        binary_full_assert_0(&cursor_1);

        let mut cursor_2 = tree.cursor().unwrap();
        binary_full_assert_0(&cursor_2);
        let mut cursor_3 = tree.cursor().unwrap();
        binary_full_assert_0(&cursor_3);

        let mut cursor_4 = tree.cursor().unwrap();
        binary_full_assert_0(&cursor_4);
        let mut cursor_5 = tree.cursor().unwrap();
        binary_full_assert_0(&cursor_5);

        assert!(cursor_0.go_to(&[0, 0]).unwrap()); // ->
        binary_full_assert_0_0(&cursor_0);
        assert!(cursor_1.go_to(&[0, 1]).unwrap()); // <-
        binary_full_assert_0_1(&cursor_1);

        assert!(cursor_2.go_to(&[0, 0, 0]).unwrap()); // ->
        binary_full_assert_0_0_0(&cursor_2);
        assert!(cursor_3.go_to(&[0, 1, 1]).unwrap()); // <-
        binary_full_assert_0_1_1(&cursor_3);

        assert!(cursor_4.go_to(&[0, 0, 0, 0]).unwrap()); // ->
        binary_full_assert_0_0_0_0(&cursor_4);
        assert!(cursor_5.go_to(&[0, 1, 1, 1]).unwrap()); // <-
        binary_full_assert_0_1_1_1(&cursor_5);

        for _ in 0..10 {
            assert!(cursor_0.jump(Direction::Right).unwrap());
            binary_full_assert_0_1(&cursor_0);
            assert!(cursor_0.jump(Direction::Right).unwrap());
            binary_full_assert_0_0(&cursor_0);
            assert!(cursor_1.jump(Direction::Left).unwrap());
            binary_full_assert_0_0(&cursor_1);
            assert!(cursor_1.jump(Direction::Left).unwrap());
            binary_full_assert_0_1(&cursor_1);

            assert!(cursor_2.jump(Direction::Right).unwrap());
            binary_full_assert_0_0_1(&cursor_2);
            assert!(cursor_2.jump(Direction::Right).unwrap());
            binary_full_assert_0_1_0(&cursor_2);
            assert!(cursor_2.jump(Direction::Right).unwrap());
            binary_full_assert_0_1_1(&cursor_2);
            assert!(cursor_2.jump(Direction::Right).unwrap());
            binary_full_assert_0_0_0(&cursor_2);
            assert!(cursor_3.jump(Direction::Left).unwrap());
            binary_full_assert_0_1_0(&cursor_3);
            assert!(cursor_3.jump(Direction::Left).unwrap());
            binary_full_assert_0_0_1(&cursor_3);
            assert!(cursor_3.jump(Direction::Left).unwrap());
            binary_full_assert_0_0_0(&cursor_3);
            assert!(cursor_3.jump(Direction::Left).unwrap());
            binary_full_assert_0_1_1(&cursor_3);

            assert!(cursor_4.jump(Direction::Right).unwrap());
            binary_full_assert_0_0_0_1(&cursor_4);
            assert!(cursor_4.jump(Direction::Right).unwrap());
            binary_full_assert_0_0_1_0(&cursor_4);
            assert!(cursor_4.jump(Direction::Right).unwrap());
            binary_full_assert_0_0_1_1(&cursor_4);
            assert!(cursor_4.jump(Direction::Right).unwrap());
            binary_full_assert_0_1_0_0(&cursor_4);
            assert!(cursor_4.jump(Direction::Right).unwrap());
            binary_full_assert_0_1_0_1(&cursor_4);
            assert!(cursor_4.jump(Direction::Right).unwrap());
            binary_full_assert_0_1_1_0(&cursor_4);
            assert!(cursor_4.jump(Direction::Right).unwrap());
            binary_full_assert_0_1_1_1(&cursor_4);
            assert!(cursor_4.jump(Direction::Right).unwrap());
            binary_full_assert_0_0_0_0(&cursor_4);
            assert!(cursor_5.jump(Direction::Left).unwrap());
            binary_full_assert_0_1_1_0(&cursor_5);
            assert!(cursor_5.jump(Direction::Left).unwrap());
            binary_full_assert_0_1_0_1(&cursor_5);
            assert!(cursor_5.jump(Direction::Left).unwrap());
            binary_full_assert_0_1_0_0(&cursor_5);
            assert!(cursor_5.jump(Direction::Left).unwrap());
            binary_full_assert_0_0_1_1(&cursor_5);
            assert!(cursor_5.jump(Direction::Left).unwrap());
            binary_full_assert_0_0_1_0(&cursor_5);
            assert!(cursor_5.jump(Direction::Left).unwrap());
            binary_full_assert_0_0_0_1(&cursor_5);
            assert!(cursor_5.jump(Direction::Left).unwrap());
            binary_full_assert_0_0_0_0(&cursor_5);
            assert!(cursor_5.jump(Direction::Left).unwrap());
            binary_full_assert_0_1_1_1(&cursor_5);
        }
    }

    #[test]
    #[serial]
    fn multiple_cursors_multiple_threads() {
        let tree = binary_tree(BINARY_FULL);

        thread::scope({
            let tree = &tree;

            |s| {
                s.spawn(|| {
                    let mut cursor_0 = tree.cursor().unwrap();
                    binary_full_assert_0(&cursor_0);
                    let mut cursor_1 = tree.cursor().unwrap();
                    binary_full_assert_0(&cursor_1);

                    assert!(cursor_0.go_to(&[0, 0]).unwrap()); // ->
                    binary_full_assert_0_0(&cursor_0);
                    assert!(cursor_1.go_to(&[0, 1]).unwrap()); // <-
                    binary_full_assert_0_1(&cursor_1);

                    for _ in 0..40 {
                        assert!(cursor_0.jump(Direction::Right).unwrap());
                        binary_full_assert_0_1(&cursor_0);
                        assert!(cursor_0.jump(Direction::Right).unwrap());
                        binary_full_assert_0_0(&cursor_0);
                        assert!(cursor_1.jump(Direction::Left).unwrap());
                        binary_full_assert_0_0(&cursor_1);
                        assert!(cursor_1.jump(Direction::Left).unwrap());
                        binary_full_assert_0_1(&cursor_1);
                    }
                });

                s.spawn(|| {
                    let mut cursor_2 = tree.cursor().unwrap();
                    binary_full_assert_0(&cursor_2);
                    let mut cursor_3 = tree.cursor().unwrap();
                    binary_full_assert_0(&cursor_3);

                    assert!(cursor_2.go_to(&[0, 0, 0]).unwrap()); // ->
                    binary_full_assert_0_0_0(&cursor_2);
                    assert!(cursor_3.go_to(&[0, 1, 1]).unwrap()); // <-
                    binary_full_assert_0_1_1(&cursor_3);

                    for _ in 0..20 {
                        assert!(cursor_2.jump(Direction::Right).unwrap());
                        binary_full_assert_0_0_1(&cursor_2);
                        assert!(cursor_2.jump(Direction::Right).unwrap());
                        binary_full_assert_0_1_0(&cursor_2);
                        assert!(cursor_2.jump(Direction::Right).unwrap());
                        binary_full_assert_0_1_1(&cursor_2);
                        assert!(cursor_2.jump(Direction::Right).unwrap());
                        binary_full_assert_0_0_0(&cursor_2);
                        assert!(cursor_3.jump(Direction::Left).unwrap());
                        binary_full_assert_0_1_0(&cursor_3);
                        assert!(cursor_3.jump(Direction::Left).unwrap());
                        binary_full_assert_0_0_1(&cursor_3);
                        assert!(cursor_3.jump(Direction::Left).unwrap());
                        binary_full_assert_0_0_0(&cursor_3);
                        assert!(cursor_3.jump(Direction::Left).unwrap());
                        binary_full_assert_0_1_1(&cursor_3);
                    }
                });

                s.spawn(|| {
                    let mut cursor_4 = tree.cursor().unwrap();
                    binary_full_assert_0(&cursor_4);
                    let mut cursor_5 = tree.cursor().unwrap();
                    binary_full_assert_0(&cursor_5);

                    assert!(cursor_4.go_to(&[0, 0, 0, 0]).unwrap()); // ->
                    binary_full_assert_0_0_0_0(&cursor_4);
                    assert!(cursor_5.go_to(&[0, 1, 1, 1]).unwrap()); // <-
                    binary_full_assert_0_1_1_1(&cursor_5);

                    for _ in 0..10 {
                        assert!(cursor_4.jump(Direction::Right).unwrap());
                        binary_full_assert_0_0_0_1(&cursor_4);
                        assert!(cursor_4.jump(Direction::Right).unwrap());
                        binary_full_assert_0_0_1_0(&cursor_4);
                        assert!(cursor_4.jump(Direction::Right).unwrap());
                        binary_full_assert_0_0_1_1(&cursor_4);
                        assert!(cursor_4.jump(Direction::Right).unwrap());
                        binary_full_assert_0_1_0_0(&cursor_4);
                        assert!(cursor_4.jump(Direction::Right).unwrap());
                        binary_full_assert_0_1_0_1(&cursor_4);
                        assert!(cursor_4.jump(Direction::Right).unwrap());
                        binary_full_assert_0_1_1_0(&cursor_4);
                        assert!(cursor_4.jump(Direction::Right).unwrap());
                        binary_full_assert_0_1_1_1(&cursor_4);
                        assert!(cursor_4.jump(Direction::Right).unwrap());
                        binary_full_assert_0_0_0_0(&cursor_4);
                        assert!(cursor_5.jump(Direction::Left).unwrap());
                        binary_full_assert_0_1_1_0(&cursor_5);
                        assert!(cursor_5.jump(Direction::Left).unwrap());
                        binary_full_assert_0_1_0_1(&cursor_5);
                        assert!(cursor_5.jump(Direction::Left).unwrap());
                        binary_full_assert_0_1_0_0(&cursor_5);
                        assert!(cursor_5.jump(Direction::Left).unwrap());
                        binary_full_assert_0_0_1_1(&cursor_5);
                        assert!(cursor_5.jump(Direction::Left).unwrap());
                        binary_full_assert_0_0_1_0(&cursor_5);
                        assert!(cursor_5.jump(Direction::Left).unwrap());
                        binary_full_assert_0_0_0_1(&cursor_5);
                        assert!(cursor_5.jump(Direction::Left).unwrap());
                        binary_full_assert_0_0_0_0(&cursor_5);
                        assert!(cursor_5.jump(Direction::Left).unwrap());
                        binary_full_assert_0_1_1_1(&cursor_5);
                    }
                });
            }
        });
    }
}
