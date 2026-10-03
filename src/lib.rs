//! ![](https://github.com/mnmun/images/blob/main/tree.png?raw=true)
//!
//! # A [`tree`] of lazily populated [`nodes`] with [`cursor-based`] traversal
//!
//! Provides a [`cursor-based`] interface for traversing [`trees`] of lazily
//! populated [`nodes`] without materializing the entire hierarchy in memory.
//! The [`nodes`] are populated only when visited by [`cursors`] and remain
//! available only while they are needed.
//!
//! Each [`node`] has an internal counter that tracks how many [`cursors`] are
//! visiting it. When a [`cursor`] descends to (visits) a [`node`], its counter
//! is incremented; when a [`cursor`] ascends from (leaves) the [`node`] -
//! decremented. When a [`cursor`] visits a [`node`] whose current counter is
//! zero, the [`node's`] [`callback`] is invoked. The resulting children remain
//! available until the last [`cursor`] leaves the [`node`], at which point the
//! counter reaches zero and children are released.
//!
//! A [`cursor`] can move in both axes through the [`tree`]:
//!
//! - Vertically, from a parent to one of its children or in the opposite
//!   direction;
//! - Horizontally, to the next or previous sibling, including siblings in an
//!   adjacent branch.
//!
//! The following diagram shows the example [`tree`] in various states
//! determined by the positions of the [`cursor`], indicated by asterisks:
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
//! In other words, [`nodes`] [`populated`] during traversal and released
//! automatically once no [`cursor`] keeps them visited.
//!
//! ## Types
//!
//! ![](https://github.com/mnmun/images/blob/main/handle_with_care.png?raw=true)
//!
//! The crate provides the following main types:
//!
//! - [`Tree`] - owns the `root` [`node`] and creates [`cursors`];
//! - [`Cursor`] - traverses and inspects the [`tree`];
//! - [`Node`] - stores [`value`], [`link`] to parent and [`links`] to
//!   currently populated children;
//! - [`Populate`] - a function used to create a [`node's`] children when a
//!   [`cursor`] visits an unvisited [`node`].
//!
//! ## Generic parameters
//!
//! ![](https://github.com/mnmun/images/blob/main/umbrella.png?raw=true)
//!
//! The following generic parameters are used in the types of this crate:
//!
//! - `Value` - the type of the [`value`] stored in each [`node`];
//! - `Error` - the type returned by the [`callback`] in the event of failure.
//!
//! ## Safety
//!
//! ![](https://github.com/mnmun/images/blob/main/slippery.png?raw=true)
//!
//! A [`tree`] can be shared between threads, and multiple [`cursors`] may
//! traverse the same [`tree`] concurrently. Access to the `root` and to each
//! [`node's`] traversal state is synchronized internally. Each [`cursor`] still
//! borrows the [`tree`] for its entire lifetime and therefore cannot outlive
//! the [`tree`] from which it was created.
//!
//! ## Example
//!
//! ![](https://github.com/mnmun/images/blob/main/bulb.png?raw=true)
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
//! Let the source data be represented by a slice of elements of type
//! `Option<&str>`, where missing [`nodes`] are denoted by `None`.
//!
//! The [`value`] stored in each [`node`] consists of the following fields:
//!
//! - `content` - a `&str` value associated with the [`node`];
//! - `source` - a reference to the source data;
//! - `position` - an index within the source data used to determine the
//!   creation of child [`nodes`].
//!
//! The `content` field is publicly accessible via the `content()` method, while
//! the `source` and `position` fields are internal states utilized only within
//! the `populate` [`callback`].
//!
//! The `populate` [`callback`] creates two or fewer child [`nodes`] based on
//! the `source` and the `position` of the parent [`node`].
//!
//! ```rust
//! use std::{borrow::Cow, ops::Range};
//! use pretty_assertions::assert_eq;
//!
//! use lazy_tree::{
//!   Tree,
//!   Node,
//!   node::Link,
//!   cursor::{Direction, Target}
//! };
//!
//! // `Value` stored in each node
//! struct Bundle<'source> {
//!     // Publicly accessible via the `content()` method
//!     content: &'source str,
//!
//!     // Internal state
//!     source: &'source[Option<&'source str>],
//!     position: usize,
//! }
//!
//! impl<'source> Bundle<'source> {
//!     pub fn content(&self) -> &'source str {
//!         &self.content
//!     }
//! }
//!
//! // The error returned by the callback
//! #[derive(Debug)]
//! enum MyError {
//!     // Indicates that the source is empty
//!     SourceIsEmpty,
//! };
//!
//! // A type alias for `Link` with the type parameters specified
//! type MyLink<'source> = Link<Bundle<'source>, MyError>;
//!
//! // A type alias for a boxed slice of `MyLink`
//! type Children<'source> = Box<[MyLink<'source>]>;
//!
//! fn populate<'source>(
//!     parent: &Bundle<'source>,
//! ) -> Result<Children<'source>, MyError> {
//!     if parent.source.is_empty() {
//!         return Err(MyError::SourceIsEmpty);
//!     }
//!
//!     if parent.position >= parent.source.len() {
//!         return Ok(Box::default());
//!     }
//!
//!     let left_child = parent.source.get(parent.position).cloned();
//!     let right_child = parent.source.get(parent.position + 1).cloned();
//!
//!     let mut children = vec![];
//!
//!     if let Some(Some(value)) = left_child {
//!         let bundle = Bundle {
//!             content: value,
//!             source: parent.source,
//!             position: parent.position * 2 + 2,
//!         };
//!
//!         children.push(Node::new(bundle, populate));
//!     }
//!
//!     if let Some(Some(value)) = right_child {
//!         let bundle = Bundle {
//!             content: value,
//!             source: parent.source,
//!             position: (parent.position + 1) * 2 + 2,
//!         };
//!
//!         children.push(Node::new(bundle, populate));
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
//!     Some("ROOT"),
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
//!     Node::new(
//!         Bundle {
//!             content: source[0].unwrap(),
//!             source: &source[1..],
//!             position: 0
//!         },
//!         populate
//!     )
//! );
//!
//! let mut cursor = tree.cursor().unwrap();
//! assert_eq!(cursor.path(), &[0]);
//! assert_eq!(cursor.value().content(), "ROOT");
//!
//! cursor.walk(Direction::Down(Target::First));
//! assert_eq!(cursor.path(), &[0, 0]);
//! assert_eq!(cursor.value().content(), "LEFT");
//!
//! cursor.walk(Direction::Right);
//! assert_eq!(cursor.path(), &[0, 1]);
//! assert_eq!(cursor.value().content(), "RIGHT");
//!
//! cursor.walk(Direction::Down(Target::First));
//! assert_eq!(cursor.path(), &[0, 1, 0]);
//! assert_eq!(cursor.value().content(), "RIGHT-RIGHT");
//! ```
//!
//! The [`lazy_json`] crate includes a JSON parser built using this crate, which
//! can serve as an usage example.
//!
//! ## Features
//!
//! ![](https://github.com/mnmun/images/blob/main/pot.png?raw=true)
//!
#![doc = document_features::document_features!()]
//!
//! ## License
//!
//! [MIT](https://github.com/mnmun/lazy_tree/blob/main/LICENSE)
//!
//! [`tree`]: crate::Tree
//! [`tree's`]: crate::Tree
//! [`trees`]: crate::Tree
//! [`node`]: crate::Node
//! [`nodes`]: crate::Node
//! [`node's`]: crate::Node
//! [`link`]: crate::node::Link
//! [`links`]: crate::node::Link
//! [`cursor`]: crate::Cursor
//! [`cursor-based`]: crate::Cursor
//! [`cursors`]: crate::Cursor
//! [`callback`]: crate::node::Callback
//! [`populated`]: crate::node::Callback
//! [`Populate`]: crate::node::Callback
//! [`value`]: crate::Node::value()
//! [`lazy_json`]: https://github.com/mnmun/lazy_json

#![allow(dead_code)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]
#![deny(rustdoc::missing_crate_level_docs)]
#![deny(rustdoc::invalid_codeblock_attributes)]
#![deny(rustdoc::invalid_html_tags)]
#![deny(rustdoc::invalid_rust_codeblocks)]
#![deny(rustdoc::unescaped_backticks)]
#![deny(rustdoc::redundant_explicit_links)]

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
        node::Link,
    };

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Bundle<'source> {
        content: &'source str,
        range: Option<Range<usize>>,
        source: &'source [Option<&'source str>],
    }

    impl<'source> Bundle<'source> {
        fn new(
            content: &'source str,
            range: impl Into<Option<Range<usize>>>,
            source: &'source [Option<&'source str>],
        ) -> Self {
            let range = range.into();
            Self {
                content,
                range,
                source,
            }
        }

        fn owned(
            content: &'source str,
            range: impl Into<Option<Range<usize>>>,
            source: &'source [Option<&'source str>],
        ) -> Cow<'source, Self> {
            Cow::Owned(Self::new(content, range, source))
        }
    }

    // type MyValue = str;
    // type MySource<'source> = Option<&'source str>;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum MyError {
        SourceIsEmpty,
        RootIsNone,
    }

    type MyLink<'source> = Link<Bundle<'source>, MyError>;
    type Children<'source> = Box<[MyLink<'source>]>;

    fn binary_tree<'source>(
        source: &'source [Option<&'source str>],
    ) -> Tree<Bundle<'source>, MyError> {
        fn populate<'source>(
            // source: impl Into<Cow<'source, [MySource<'source>]>>,
            parent: &Bundle<'source>,
        ) -> Result<Children<'source>, MyError> {
            let range = if let Some(range) = &parent.range {
                range
            } else {
                return Ok(Box::default());
            };

            // let source = source.into();
            let source = parent.source;

            let left_child = source.get(range.start).cloned();
            let right_child = source.get(range.start + 1).cloned();

            let mut children = vec![];

            if let Some(Some(content)) = left_child {
                children.push(Node::new(
                    Bundle::new(
                        content,
                        {
                            let start = range.start * 2 + 2;
                            if source.len() < start {
                                None
                            } else {
                                Some(start..source.len())
                            }
                        },
                        source,
                    ),
                    populate,
                ));
            }

            if let Some(Some(value)) = right_child {
                children.push(Node::new(
                    Bundle::new(
                        value,
                        {
                            let start = (range.start + 1) * 2 + 2;
                            if source.len() < start {
                                None
                            } else {
                                Some(start..source.len())
                            }
                        },
                        source,
                    ),
                    populate,
                ));
            }

            Ok(children.into_boxed_slice())
        }

        Tree::new(Node::new(
            Bundle::new("ROOT", 0..source.len(), source),
            populate,
        ))
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
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "ROOT");
        assert_eq!(
            cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            ["L", "R"]
        );
    }

    fn binary_0_assert_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "L");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            ["LL", "LR"]
        );
    }

    fn binary_0_assert_0_1<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 1]);
        assert_eq!(Node::from_link(cursor.link).value().content, "R");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            ["RL", "RR",]
        );
    }

    fn binary_0_assert_0_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "LL");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            Vec::<&str>::default()
        );
    }

    fn binary_0_assert_0_0_1<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 1]);
        assert_eq!(Node::from_link(cursor.link).value().content, "LR");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            ["LRL"]
        );
    }

    fn binary_0_assert_0_1_0<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 1, 0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "RL");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            ["RLL" /*, "RLR" */]
        );
    }

    fn binary_0_assert_0_1_1<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 1, 1]);
        assert_eq!(Node::from_link(cursor.link).value().content, "RR");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            Vec::<&str>::default()
        );
    }

    fn binary_0_assert_0_0_1_0<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 1, 0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "LRL");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            Vec::<&str>::default()
        );
    }

    fn binary_0_assert_0_1_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 1, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "RLL");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            Vec::<&str>::default()
        );
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
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "ROOT");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            ["L"]
        );
    }

    fn binary_1_assert_0_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "LL");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            Vec::<&str>::default()
        );
    }

    fn binary_1_assert_0_0_1<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 1]);
        assert_eq!(Node::from_link(cursor.link).value().content, "LR");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            ["LRL"]
        );
    }

    fn binary_1_assert_0_0_1_0<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 1, 0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "LRL");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            Vec::<&str>::default()
        );
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
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "ROOT");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            ["L"]
        );
    }

    fn binary_2_assert_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "L");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            ["LL", "LR"]
        );
    }

    fn binary_2_assert_0_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "LL");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            ["LLL"]
        );
    }

    fn binary_2_assert_0_0_1<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 1]);
        assert_eq!(Node::from_link(cursor.link).value().content, "LR");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            Vec::<&str>::default()
        );
    }

    fn binary_2_assert_0_0_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "LLL");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            Vec::<&str>::default()
        );
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
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "ROOT");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            ["L"]
        );
    }

    fn binary_3_assert_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "L");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            ["LL", "LR"]
        );
    }

    fn binary_3_assert_0_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "LL");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            ["LLL"]
        );
    }

    fn binary_3_assert_0_0_1<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 1]);
        assert_eq!(Node::from_link(cursor.link).value().content, "LR");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            ["LRL"]
        );
    }

    fn binary_3_assert_0_0_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "LLL");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            ["LLLL"]
        );
    }

    fn binary_3_assert_0_0_1_0<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 1, 0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "LRL");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            Vec::<&str>::default()
        );
    }

    fn binary_3_assert_0_0_0_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 0, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "LLLL");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            Vec::<&str>::default()
        );
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
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "ROOT");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            ["L"]
        );
    }

    fn binary_4_assert_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "L");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            ["LL", "LR"]
        );
    }

    fn binary_4_assert_0_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "LL");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            ["LLL"]
        );
    }

    fn binary_4_assert_0_0_1<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 1]);
        assert_eq!(Node::from_link(cursor.link).value().content, "LR");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            ["LRL"]
        );
    }

    fn binary_4_assert_0_0_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "LLL");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            Vec::<&str>::default()
        );
    }

    fn binary_4_assert_0_0_1_0<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 1, 0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "LRL");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            ["LRLL"]
        );
    }

    fn binary_4_assert_0_0_1_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 1, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "LRLL");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            Vec::<&str>::default()
        );
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
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "ROOT");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            ["L"]
        );
    }

    fn list_assert_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "L");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            ["LL"]
        );
    }

    fn list_assert_0_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "LL");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            ["LLL"]
        );
    }

    fn list_assert_0_0_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "LLL");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            ["LLLL"]
        );
    }

    fn list_assert_0_0_0_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 0, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "LLLL");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            Vec::<&str>::default()
        );
    }

    /// ```plain
    /// ROOT
    /// ```
    const EMPTY: &[Option<&str>] = &[None, None];

    fn empty_assert_0<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "ROOT");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            Vec::<&str>::default()
        );
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
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "ROOT");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            ["L", "R"]
        );
    }

    fn binary_full_assert_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "L");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            ["LL", "LR"]
        );
    }

    fn binary_full_assert_0_1<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 1]);
        assert_eq!(Node::from_link(cursor.link).value().content, "R");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            ["RL", "RR",]
        );
    }

    fn binary_full_assert_0_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "LL");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            ["LLL", "LLR"]
        );
    }

    fn binary_full_assert_0_0_1<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 1]);
        assert_eq!(Node::from_link(cursor.link).value().content, "LR");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            ["LRL", "LRR"]
        );
    }

    fn binary_full_assert_0_1_0<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 1, 0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "RL");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            ["RLL", "RLR"]
        );
    }

    fn binary_full_assert_0_1_1<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 1, 1]);
        assert_eq!(Node::from_link(cursor.link).value().content, "RR");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            ["RRL", "RRR"]
        );
    }

    fn binary_full_assert_0_0_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "LLL");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            Vec::<&str>::default()
        );
    }

    fn binary_full_assert_0_0_0_1<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 0, 1]);
        assert_eq!(Node::from_link(cursor.link).value().content, "LLR");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            Vec::<&str>::default()
        );
    }

    fn binary_full_assert_0_0_1_0<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 1, 0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "LRL");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            Vec::<&str>::default()
        );
    }

    fn binary_full_assert_0_0_1_1<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 0, 1, 1]);
        assert_eq!(Node::from_link(cursor.link).value().content, "LRR");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            Vec::<&str>::default()
        );
    }

    fn binary_full_assert_0_1_0_0<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 1, 0, 0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "RLL");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            Vec::<&str>::default()
        );
    }

    fn binary_full_assert_0_1_0_1<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 1, 0, 1]);
        assert_eq!(Node::from_link(cursor.link).value().content, "RLR");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            Vec::<&str>::default()
        );
    }

    fn binary_full_assert_0_1_1_0<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 1, 1, 0]);
        assert_eq!(Node::from_link(cursor.link).value().content, "RRL");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            Vec::<&str>::default()
        );
    }

    fn binary_full_assert_0_1_1_1<'tree, 'source>(
        cursor: &Cursor<'tree, Bundle<'source>, MyError>,
    ) {
        assert_eq!(cursor.path(), &[0, 1, 1, 1]);
        assert_eq!(Node::from_link(cursor.link).value().content, "RRR");
        assert_eq!(
            *cursor
                .children()
                .iter()
                .map(|child| child.value().content)
                .collect::<Vec<_>>(),
            Vec::<&str>::default()
        );
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
