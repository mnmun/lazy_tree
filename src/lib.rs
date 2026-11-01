//! Makes it possible to traverse a [`tree`] of lazily-created [`nodes`] using
//! [`cursors`] without having to create the entire [`tree`] structure in
//! advance. [`Nodes`] are [created] automatically using [callbacks] when
//! the [`cursor`] needs to access them and are deleted when they are no longer
//! used.
//!
//! Used in [lazy_json].
//!
//! ## [`Tree`]
//!
//! ![](https://github.com/mnmun/images/blob/main/tree.png?raw=true)
//!
//! [`Tree`] is a wrapper aroung the [`link`] to the `root` [`node`] with the
//! ability to create [`cursors`] for traversing hierarchical structure of
//! lazily-created [`nodes`]. [`Tree`] may have multiple [`cursors`] at the
//! same time, pointing to different [`nodes`] and moving independently of each
//! other.
//!
//! ## [`Cursor`]
//!
//! ![](https://github.com/mnmun/images/blob/main/index_left.png?raw=true)
//!
//! [`Cursor`] is a tool for traversing a [`tree`] of lazily-created [`nodes`].
//! It can move between [`nodes`] within the [`tree`] and provides access to
//! the `value` stored in the [`node`] it currently points to. As it moves, the
//! [`cursor`] triggers the [creation] of any [`nodes`] it need to access, as
//! well as the deletion of [`nodes`] that are no longer in use.
//!
//! ## [Callbacks]
//!
//! ![](https://github.com/mnmun/images/blob/main/telephone.png?raw=true)
//!
//! Each [`node`] in the [`tree`] contains two [callbacks], which are invoked
//! automatically by the [`cursor`] as it moves through the [`tree`]:
//!
//! - [`Total`] - invoked on the [`node`] to which the [`cursor`] has just
//!   moved. It ensures the [`query`] callback functions correctly by
//!   calculating the `total` number of child [`nodes`] for a given parent
//!   [`node`]. If this calculation has already been performed for this
//!   [`node`], it is not repeated;
//! - [`Query`] - invoked on a parent [`node`] when the [`cursor`] moves to its
//!   child [`node`] that has not yet been created. It handles the creation of
//!   a new child [`node`] only when access to that [`node`] is actually
//!   required.
//!
//! When a [`cursor`] is created, the [`total`] callback is called on the `root`
//! [`node`] as if the [`cursor`] had moved to it from the parent [`node`].
//!
//! In this way, a developer can define the logic for constructing a [`tree`] of
//! lazily-created [`nodes`] tailored to their specific use case by implementing
//! these [callbacks].
//!
//! ## Example
//!
//! ![](https://github.com/mnmun/images/blob/main/bulb.png?raw=true)
//!
//! The following example constructs a lazily-created binary [`tree`] from
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
//! Let the `source` data be represented by a slice of elements of type
//! `Option<&str>`, where missing [`nodes`] are denoted by `None`:
//!
//! ```rust
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
//!     /* [0]          */ Some("ROOT"),
//!     /* [0, 0]       */ Some("LEFT"),
//!     /* [0, 1]       */ Some("RIGHT"),
//!     /* [0, 0, 0]    */ Some("LEFT-LEFT"),
//!     /* [0, 0, 1]    */ Some("LEFT-RIGHT"),
//!     /* [0, 1, 0]    */ None, // RIGHT-LEFT
//!     /* [0, 1, 1]    */ Some("RIGHT-RIGHT"),
//!     /* [0, 0, 0, 0] */ Some("LEFT-LEFT-LEFT"),
//!     /* [0, 0, 0, 1] */ None, // LEFT-LEFT-RIGHT
//! ];
//! ```
//!
//! The `value` stored in each [`node`] consists of the following fields:
//!
//! - `content` - the `&str` associated with the [`node`];
//! - `source` - a shared reference to the `source` data;
//! - `left_child_index` - the index of an element in the `source` data that is
//!   associated with the left child [`node`] of this parent [`node`].
//!
//! The `content` field is publicly accessible via a [`cursor`] using the
//! `Deref` trait, while the `source` and `left_child_index` fields are internal
//! states utilized only within the `total` and `query` [callbacks]:
//!
//! ```rust
//! # use std::ops::Deref;
//! // `Value` stored in each node
//! struct Bundle<'source> {
//!     // Publicly accessible via the `Deref` trait
//!     content: &'source str,
//!
//!     // Internal state
//!     source: &'source[Option<&'source str>],
//!     left_child_index: usize,
//! }
//!
//! // The `content` field can be accessed via cursor dereferencing: `**cursor`
//! impl<'source> Deref for Bundle<'source> {
//!     type Target = &'source str;
//!
//!     fn deref(&self) -> &Self::Target {
//!         &self.content
//!     }
//! }
//!
//! impl<'source> Bundle<'source> {
//!     // Method for creating a new bundle
//!     fn new(
//!         content: &'source str,
//!         source: &'source [Option<&'source str>],
//!         left_child_index: usize,
//!     ) -> Self {
//!         Self {
//!             content,
//!             source,
//!             left_child_index,
//!         }
//!     }
//! }
//! ```
//!
//! We need to specify a type to be used for errors returned by the `total` and
//! `query` [callbacks]. Since there are no scenarios in this simple example
//! where these [callbacks] might fail, this type will not be actually used; it
//! is included here solely for demonstration purposes:
//!
//! ```rust
//! // The errors returned by the callbacks
//! #[derive(Debug)]
//! enum MyError {
//!     TotalError,
//!     QueryError,
//! };
//! ```
//!
//! The `total` [callback](crate::callbacks::Total) determines the `total`
//! number of child [`nodes`] that this parent [`node`] can have:
//!
//! ```rust
//! # struct Bundle<'source> {
//! #     content: &'source str,
//! #     source: &'source[Option<&'source str>],
//! #     left_child_index: usize,
//! # }
//! # #[derive(Debug)]
//! # enum MyError {
//! #     TotalError,
//! #     QueryError,
//! # };
//! fn total<'source>(parent: &Bundle<'source>) -> Result<usize, MyError> {
//!     // The callback can return an error if necessary:
//!     // if condition {
//!     //     return Err(MyError::TotalError);
//!     // }
//!
//!     let mut total = 0;
//!
//!     if let Some(Some(_)) = parent.source.get(parent.left_child_index) {
//!         total += 1;
//!     }
//!
//!     if let Some(Some(_)) = parent.source.get(parent.left_child_index + 1) {
//!         total += 1;
//!     }
//!
//!     Ok(total)
//! }
//! ```
//!
//! The `query` [callback](crate::callbacks::Query) creates a child [`node`]
//! [`builder`] for the given `index` in the range `0..total`:
//!
//! ```rust
//! # use lazy_tree::node;
//! # struct Bundle<'source> {
//! #     content: &'source str,
//! #     source: &'source[Option<&'source str>],
//! #     left_child_index: usize,
//! # }
//! # impl<'source> Bundle<'source> {
//! #     fn new(
//! #         content: &'source str,
//! #         source: &'source [Option<&'source str>],
//! #         left_child_index: usize,
//! #     ) -> Self {
//! #         Self {
//! #             content,
//! #             source,
//! #             left_child_index,
//! #         }
//! #     }
//! # }
//! # #[derive(Debug)]
//! # enum MyError {
//! #     TotalError,
//! #     QueryError,
//! # };
//! # fn total<'source>(parent: &Bundle<'source>) -> Result<usize, MyError> {
//! #     Err(MyError::TotalError)
//! # }
//! fn query<'source>(
//!     parent: &Bundle<'source>,
//!     index: usize,
//! ) -> Result<node::Builder<Bundle<'source>, MyError>, MyError> {
//!     // The callback can return an error if necessary:
//!     // if condition {
//!     //     return Err(MyError::QueryError);
//!     // }
//!
//!     let bundle = match (
//!         parent
//!             .source
//!             .get(parent.left_child_index)
//!             .cloned()
//!             .flatten(),
//!         parent
//!             .source
//!             .get(parent.left_child_index + 1)
//!             .cloned()
//!             .flatten(),
//!     ) {
//!         (Some(left), _) if index == 0 => Bundle::new(
//!             left,
//!             parent.source,
//!             parent.left_child_index * 2 + 1,
//!         ),
//!         (None, Some(right)) if index == 0 => Bundle::new(
//!             right,
//!             parent.source,
//!             (parent.left_child_index + 1) * 2 + 1,
//!         ),
//!         (Some(_), Some(right)) if index == 1 => Bundle::new(
//!             right,
//!             parent.source,
//!             (parent.left_child_index + 1) * 2 + 1,
//!         ),
//!         _ => unreachable!(),
//!     };
//!
//!     Ok(node::Builder::new(bundle, total, query))
//! }
//! ```
//!
//! Simple function for creating a [`tree`], with validation of the `source`
//! data:
//!
//! ```rust
//! # use lazy_tree::{
//! #     Tree,
//! #     node,
//! # };
//! # struct Bundle<'source> {
//! #     content: &'source str,
//! #     source: &'source[Option<&'source str>],
//! #     left_child_index: usize,
//! # }
//! # impl<'source> Bundle<'source> {
//! #     fn new(
//! #         content: &'source str,
//! #         source: &'source [Option<&'source str>],
//! #         left_child_index: usize,
//! #     ) -> Self {
//! #         Self {
//! #             content,
//! #             source,
//! #             left_child_index,
//! #         }
//! #     }
//! # }
//! # #[derive(Debug)]
//! # enum MyError {
//! #     TotalError,
//! #     QueryError,
//! # };
//! # fn total<'source>(parent: &Bundle<'source>) -> Result<usize, MyError> {
//! #     Err(MyError::TotalError)
//! # }
//! # fn query<'source>(
//! #     parent: &Bundle<'source>,
//! #     index: usize,
//! # ) -> Result<node::Builder<Bundle<'source>, MyError>, MyError> {
//! #     Err(MyError::QueryError)
//! # }
//! fn make_tree<'source>(
//!     source: &'source [Option<&'source str>]
//! ) -> Tree<Bundle<'source>, MyError> {
//!     if source.is_empty() {
//!         panic!("Source is empty");
//!     }
//!
//!     if source[0].is_none() {
//!         panic!("Root is None");
//!     }
//!
//!     Tree::new(node::Builder::new(
//!         Bundle::new(source[0].unwrap(), source, 1),
//!         total,
//!         query,
//!     ))
//! }
//! ```
//!
//! And finally, the binary [`tree`] can be traversed using the [`cursor`]:
//!
//! ```rust
//! # use std::ops::Deref;
//! # use pretty_assertions::assert_eq;
//! # use lazy_tree::{
//! #     Tree,
//! #     node,
//! #     cursor::{Direction, Target}
//! # };
//! # struct Bundle<'source> {
//! #     content: &'source str,
//! #     source: &'source[Option<&'source str>],
//! #     left_child_index: usize,
//! # }
//! # impl<'source> Deref for Bundle<'source> {
//! #     type Target = &'source str;
//! #     fn deref(&self) -> &Self::Target {
//! #         &self.content
//! #     }
//! # }
//! # impl<'source> Bundle<'source> {
//! #     fn new(
//! #         content: &'source str,
//! #         source: &'source [Option<&'source str>],
//! #         left_child_index: usize,
//! #     ) -> Self {
//! #         Self {
//! #             content,
//! #             source,
//! #             left_child_index,
//! #         }
//! #     }
//! # }
//! # #[derive(Debug)]
//! # enum MyError {
//! #     TotalError,
//! #     QueryError,
//! # };
//! # fn total<'source>(parent: &Bundle<'source>) -> Result<usize, MyError> {
//! #     let mut total = 0;
//! #     if let Some(Some(_)) = parent.source.get(parent.left_child_index) {
//! #         total += 1;
//! #     }
//! #     if let Some(Some(_)) = parent.source.get(parent.left_child_index + 1) {
//! #         total += 1;
//! #     }
//! #     Ok(total)
//! # }
//! # fn query<'source>(
//! #     parent: &Bundle<'source>,
//! #     index: usize,
//! # ) -> Result<node::Builder<Bundle<'source>, MyError>, MyError> {
//! #     let bundle = match (
//! #         parent
//! #             .source
//! #             .get(parent.left_child_index)
//! #             .cloned()
//! #             .flatten(),
//! #         parent
//! #             .source
//! #             .get(parent.left_child_index + 1)
//! #             .cloned()
//! #             .flatten(),
//! #     ) {
//! #         (Some(left), _) if index == 0 => Bundle::new(
//! #             left,
//! #             parent.source,
//! #             parent.left_child_index * 2 + 1,
//! #         ),
//! #         (None, Some(right)) if index == 0 => Bundle::new(
//! #             right,
//! #             parent.source,
//! #             (parent.left_child_index + 1) * 2 + 1,
//! #         ),
//! #         (Some(_), Some(right)) if index == 1 => Bundle::new(
//! #             right,
//! #             parent.source,
//! #             (parent.left_child_index + 1) * 2 + 1,
//! #         ),
//! #         _ => unreachable!(),
//! #     };
//! #     Ok(node::Builder::new(bundle, total, query))
//! # }
//! # fn make_tree<'source>(
//! #     source: &'source [Option<&'source str>]
//! # ) -> Tree<Bundle<'source>, MyError> {
//! #     if source.is_empty() {
//! #         panic!("Source is empty");
//! #     }
//! #     if source[0].is_none() {
//! #         panic!("Root is None");
//! #     }
//! #     Tree::new(node::Builder::new(
//! #         Bundle::new(source[0].unwrap(), source, 1),
//! #         total,
//! #         query,
//! #     ))
//! # }
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
//!     /* [0]          */ Some("ROOT"),
//!     /* [0, 0]       */ Some("LEFT"),
//!     /* [0, 1]       */ Some("RIGHT"),
//!     /* [0, 0, 0]    */ Some("LEFT-LEFT"),
//!     /* [0, 0, 1]    */ Some("LEFT-RIGHT"),
//!     /* [0, 1, 0]    */ None, // RIGHT-LEFT
//!     /* [0, 1, 1]    */ Some("RIGHT-RIGHT"),
//!     /* [0, 0, 0, 0] */ Some("LEFT-LEFT-LEFT"),
//!     /* [0, 0, 0, 1] */ None, // LEFT-LEFT-RIGHT
//! ];
//!
//! let tree = make_tree(source);
//!
//! let mut cursor = tree.cursor().unwrap();
//! assert_eq!(cursor.path(), &[0]);
//! assert_eq!(**cursor, "ROOT");
//!
//! cursor.walk(Direction::Down(Target::First));
//! assert_eq!(cursor.path(), &[0, 0]);
//! assert_eq!(**cursor, "LEFT");
//!
//! cursor.walk(Direction::Right);
//! assert_eq!(cursor.path(), &[0, 1]);
//! assert_eq!(**cursor, "RIGHT");
//!
//! cursor.walk(Direction::Down(Target::First));
//! assert_eq!(cursor.path(), &[0, 1, 0]);
//! assert_eq!(**cursor, "RIGHT-RIGHT");
//! ```
//!
//! The complete code:
//!
//! ```rust
//! use std::ops::Deref;
//! use pretty_assertions::assert_eq;
//!
//! use lazy_tree::{
//!     Tree,
//!     node,
//!     cursor::{Direction, Target}
//! };
//!
//! // `Value` stored in each node
//! struct Bundle<'source> {
//!     // Publicly accessible via the `Deref` trait
//!     content: &'source str,
//!
//!     // Internal state
//!     source: &'source[Option<&'source str>],
//!     left_child_index: usize,
//! }
//!
//! // The `content` field can be accessed via cursor dereferencing: `**cursor`
//! impl<'source> Deref for Bundle<'source> {
//!     type Target = &'source str;
//!
//!     fn deref(&self) -> &Self::Target {
//!         &self.content
//!     }
//! }
//!
//! impl<'source> Bundle<'source> {
//!     // Method for creating a new bundle
//!     fn new(
//!         content: &'source str,
//!         source: &'source [Option<&'source str>],
//!         left_child_index: usize,
//!     ) -> Self {
//!         Self {
//!             content,
//!             source,
//!             left_child_index,
//!         }
//!     }
//! }
//!
//! // The errors returned by the callbacks
//! #[derive(Debug)]
//! enum MyError {
//!     TotalError,
//!     QueryError,
//! };
//!
//! fn total<'source>(parent: &Bundle<'source>) -> Result<usize, MyError> {
//!     // The callback can return an error if necessary:
//!     // if condition {
//!     //     return Err(MyError::TotalError);
//!     // }
//!
//!     let mut total = 0;
//!
//!     if let Some(Some(_)) = parent.source.get(parent.left_child_index) {
//!         total += 1;
//!     }
//!
//!     if let Some(Some(_)) = parent.source.get(parent.left_child_index + 1) {
//!         total += 1;
//!     }
//!
//!     Ok(total)
//! }
//!
//! fn query<'source>(
//!     parent: &Bundle<'source>,
//!     index: usize,
//! ) -> Result<node::Builder<Bundle<'source>, MyError>, MyError> {
//!     // if condition {
//!     //     return Err(MyError::QueryError);
//!     // }
//!
//!     let bundle = match (
//!         parent
//!             .source
//!             .get(parent.left_child_index)
//!             .cloned()
//!             .flatten(),
//!         parent
//!             .source
//!             .get(parent.left_child_index + 1)
//!             .cloned()
//!             .flatten(),
//!     ) {
//!         (Some(left), _) if index == 0 => Bundle::new(
//!             left,
//!             parent.source,
//!             parent.left_child_index * 2 + 1,
//!         ),
//!         (None, Some(right)) if index == 0 => Bundle::new(
//!             right,
//!             parent.source,
//!             (parent.left_child_index + 1) * 2 + 1,
//!         ),
//!         (Some(_), Some(right)) if index == 1 => Bundle::new(
//!             right,
//!             parent.source,
//!             (parent.left_child_index + 1) * 2 + 1,
//!         ),
//!         _ => unreachable!(),
//!     };
//!
//!     Ok(node::Builder::new(bundle, total, query))
//! }
//!
//! fn make_tree<'source>(
//!     source: &'source [Option<&'source str>]
//! ) -> Tree<Bundle<'source>, MyError> {
//!     if source.is_empty() {
//!         panic!("Source is empty");
//!     }
//!
//!     if source[0].is_none() {
//!         panic!("Root is None");
//!     }
//!
//!     Tree::new(node::Builder::new(
//!         Bundle::new(source[0].unwrap(), source, 1),
//!         total,
//!         query,
//!     ))
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
//!     /* [0]          */ Some("ROOT"),
//!     /* [0, 0]       */ Some("LEFT"),
//!     /* [0, 1]       */ Some("RIGHT"),
//!     /* [0, 0, 0]    */ Some("LEFT-LEFT"),
//!     /* [0, 0, 1]    */ Some("LEFT-RIGHT"),
//!     /* [0, 1, 0]    */ None, // RIGHT-LEFT
//!     /* [0, 1, 1]    */ Some("RIGHT-RIGHT"),
//!     /* [0, 0, 0, 0] */ Some("LEFT-LEFT-LEFT"),
//!     /* [0, 0, 0, 1] */ None, // LEFT-LEFT-RIGHT
//! ];
//!
//! let tree = make_tree(source);
//!
//! let mut cursor = tree.cursor().unwrap();
//! assert_eq!(cursor.path(), &[0]);
//! assert_eq!(**cursor, "ROOT");
//!
//! cursor.walk(Direction::Down(Target::First));
//! assert_eq!(cursor.path(), &[0, 0]);
//! assert_eq!(**cursor, "LEFT");
//!
//! cursor.walk(Direction::Right);
//! assert_eq!(cursor.path(), &[0, 1]);
//! assert_eq!(**cursor, "RIGHT");
//!
//! cursor.walk(Direction::Down(Target::First));
//! assert_eq!(cursor.path(), &[0, 1, 0]);
//! assert_eq!(**cursor, "RIGHT-RIGHT");
//! ```
//!
//! ## License
//!
//! [MIT](https://github.com/mnmun/lazy_tree/blob/main/LICENSE)
//!
//! [`tree`]: crate::Tree
//! [`tree's`]: crate::Tree
//! [`trees`]: crate::Tree
//! [`node`]: crate::Node
//! [`node's`]: crate::Node
//! [`nodes`]: crate::Node
//! [`builder`]: crate::node::Builder
//! [`link`]: crate::node::Link
//! [`links`]: crate::node::Link
//! [`cursor`]: crate::Cursor
//! [`cursors`]: crate::Cursor
//! [callbacks]: crate::callbacks
//! [Callbacks]: crate::callbacks
//! [`Total`]: crate::callbacks::Total
//! [`total`]: crate::callbacks::Total
//! [`Query`]: crate::callbacks::Query
//! [`query`]: crate::callbacks::Query
//! [creation]: crate::callbacks::Query
//! [created]: crate::callbacks::Query
//! [lazy_json]: https://github.com/mnmun/lazy_json

#![allow(dead_code)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]
#![deny(rustdoc::missing_crate_level_docs)]
#![deny(rustdoc::invalid_codeblock_attributes)]
#![deny(rustdoc::invalid_html_tags)]
#![deny(rustdoc::invalid_rust_codeblocks)]
#![deny(rustdoc::unescaped_backticks)]
#![deny(rustdoc::redundant_explicit_links)]

pub mod callbacks;
pub mod cursor;
pub mod node;
pub mod tree;

pub use cursor::Cursor;
pub use node::Node;
pub use tree::Tree;

#[cfg(test)]
mod tests {

    use std::{ops::Deref, thread};

    use pretty_assertions::assert_eq;
    use serial_test::serial;

    use crate::{
        Node, Tree,
        cursor::{Direction, Target},
        node,
    };

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Bundle<'source> {
        content: &'source str,

        source: &'source [Option<&'source str>],
        left_child_index: usize,
    }

    impl<'source> Deref for Bundle<'source> {
        type Target = &'source str;

        fn deref(&self) -> &Self::Target {
            &self.content
        }
    }

    impl<'source> Bundle<'source> {
        fn new(
            content: &'source str,
            left_child_index: usize,
            source: &'source [Option<&'source str>],
        ) -> Self {
            Self {
                content,
                left_child_index,
                source,
            }
        }
    }

    fn binary_tree<'source>(
        source: &'source [Option<&'source str>],
    ) -> Tree<Bundle<'source>, ()> {
        fn total<'source>(parent: &Bundle<'source>) -> Result<usize, ()> {
            let mut total = 0;

            if let Some(Some(_)) = parent.source.get(parent.left_child_index) {
                total += 1;
            }

            if let Some(Some(_)) =
                parent.source.get(parent.left_child_index + 1)
            {
                total += 1;
            }

            Ok(total)
        }

        fn query<'source>(
            parent: &Bundle<'source>,
            index: usize,
        ) -> Result<node::Builder<Bundle<'source>, ()>, ()> {
            let bundle = match (
                parent
                    .source
                    .get(parent.left_child_index)
                    .cloned()
                    .flatten(),
                parent
                    .source
                    .get(parent.left_child_index + 1)
                    .cloned()
                    .flatten(),
            ) {
                (Some(left), _) if index == 0 => Bundle::new(
                    left,
                    parent.left_child_index * 2 + 1,
                    parent.source,
                ),
                (None, Some(right)) if index == 0 => Bundle::new(
                    right,
                    (parent.left_child_index + 1) * 2 + 1,
                    parent.source,
                ),
                (Some(_), Some(right)) if index == 1 => Bundle::new(
                    right,
                    (parent.left_child_index + 1) * 2 + 1,
                    parent.source,
                ),
                _ => unreachable!(),
            };

            Ok(node::Builder::new(bundle, total, query))
        }

        if source.is_empty() {
            panic!("Source is empty");
        }

        if source[0].is_none() {
            panic!("Root is None");
        }

        Tree::new(node::Builder::new(
            Bundle::new(source[0].unwrap(), 1, source),
            total,
            query,
        ))
    }

    macro_rules! assert_children {
        ($left:expr, $right:expr) => {{
            let left = $left;
            let right = $right;

            assert_eq!(left.len(), right.len());
            left.values().zip(right.iter()).for_each(
                |(left, right): (&&Node<Bundle<'_>, ()>, &&str)| {
                    assert_eq!(&left.value.content, right);
                },
            );
        }};
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
        /* [0]          */ Some("ROOT"),
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
        /* [0]          */ Some("ROOT"),
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
        /* [0]          */ Some("ROOT"),
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
        /* [0]             */ Some("ROOT"),
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
        /* [0]             */ Some("ROOT"),
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
        /* [0]             */ Some("ROOT"),
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

    /// ```plain
    /// ROOT
    /// ```
    const EMPTY: &[Option<&str>] = &[Some("ROOT"), None, None];

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
        /* [0]          */ Some("ROOT"),
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

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    fn go_to() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(**cursor, "L");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 1]).unwrap());
        assert_eq!(cursor.path(), &[0, 1]);
        assert_eq!(**cursor, "R");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(**cursor, "LL");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0, 1]).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 1]);
        assert_eq!(**cursor, "LR");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 1, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 1, 0]);
        assert_eq!(**cursor, "RL");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 1, 1]).unwrap());
        assert_eq!(cursor.path(), &[0, 1, 1]);
        assert_eq!(**cursor, "RR");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0, 1, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 1, 0]);
        assert_eq!(**cursor, "LRL");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 1, 0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 1, 0, 0]);
        assert_eq!(**cursor, "RLL");
        assert_children!(cursor.node().children(), &[]);

        assert!(!cursor.go_to(&[0, 0, 0, 0, 0, 0, 0]).unwrap());
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    fn wrong_go_to() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(**cursor, "L");
        assert_children!(cursor.node().children(), &[]);

        assert!(!cursor.go_to(&[0, 0, 2]).unwrap());
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(**cursor, "L");
        assert_children!(cursor.node().children(), &[]);

        assert!(!cursor.go_to(&[0, 1, 2]).unwrap());
        assert_eq!(cursor.path(), &[0, 1]);
        assert_eq!(**cursor, "R");
        assert_children!(cursor.node().children(), &[]);

        assert!(!cursor.go_to(&[0, 3, 0]).unwrap());
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(!cursor.go_to(&[3, 0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Jump from ROOT to LL
    fn jump_up_from_root() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Up).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(**cursor, "LL");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Jump from LL to L
    fn jump_up() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(**cursor, "LL");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Up).unwrap());
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(**cursor, "L");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`EMPTY`]
    fn jump_up_from_root_in_empty_tree() {
        let tree = binary_tree(EMPTY);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Up).unwrap());
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Jump from RLL to ROOT
    fn jump_down_from_leaf() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 1, 0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 1, 0, 0]);
        assert_eq!(**cursor, "RLL");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Down(Target::First)).unwrap());
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 1, 0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 1, 0, 0]);
        assert_eq!(**cursor, "RLL");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Down(Target::Last)).unwrap());
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 1, 0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 1, 0, 0]);
        assert_eq!(**cursor, "RLL");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Down(Target::Index(0))).unwrap());
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 1, 0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 1, 0, 0]);
        assert_eq!(**cursor, "RLL");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Down(Target::Index(2))).unwrap());
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);
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
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(**cursor, "L");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Down(Target::First)).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(**cursor, "LL");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(**cursor, "L");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Down(Target::Last)).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 1]);
        assert_eq!(**cursor, "LR");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(**cursor, "L");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Down(Target::Index(1))).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 1]);
        assert_eq!(**cursor, "LR");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(**cursor, "L");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Down(Target::Index(2))).unwrap());
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`EMPTY`]
    fn jump_down_from_root_in_empty_tree() {
        let tree = binary_tree(EMPTY);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Down(Target::First)).unwrap());
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Down(Target::Last)).unwrap());
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Down(Target::Index(0))).unwrap());
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Down(Target::Index(2))).unwrap());
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Attempt to walk from ROOT to LL
    fn walk_up_from_root() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(!cursor.walk(Direction::Up).unwrap());
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Move from LL to L
    fn walk_up() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(**cursor, "LL");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.walk(Direction::Up).unwrap());
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(**cursor, "L");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`EMPTY`]
    fn walk_up_from_root_in_empty_tree() {
        let tree = binary_tree(EMPTY);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(!cursor.walk(Direction::Up).unwrap());
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Attempt to walk from RLL to ROOT
    fn walk_down_from_leaf() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 1, 0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 1, 0, 0]);
        assert_eq!(**cursor, "RLL");
        assert_children!(cursor.node().children(), &[]);

        assert!(!cursor.walk(Direction::Down(Target::First)).unwrap());
        assert_eq!(cursor.path(), &[0, 1, 0, 0]);
        assert_eq!(**cursor, "RLL");
        assert_children!(cursor.node().children(), &[]);
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
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(**cursor, "L");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.walk(Direction::Down(Target::First)).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(**cursor, "LL");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(**cursor, "L");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.walk(Direction::Down(Target::Last)).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 1]);
        assert_eq!(**cursor, "LR");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(**cursor, "L");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.walk(Direction::Down(Target::Index(1))).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 1]);
        assert_eq!(**cursor, "LR");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(**cursor, "L");
        assert_children!(cursor.node().children(), &[]);

        assert!(!cursor.walk(Direction::Down(Target::Index(2))).unwrap());
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(**cursor, "L");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`EMPTY`]
    fn walk_down_from_root_in_empty_tree() {
        let tree = binary_tree(EMPTY);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(!cursor.walk(Direction::Down(Target::First)).unwrap());
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(!cursor.walk(Direction::Down(Target::Last)).unwrap());
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(!cursor.walk(Direction::Down(Target::Index(0))).unwrap());
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(!cursor.walk(Direction::Down(Target::Index(2))).unwrap());
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Jump from LL to RR
    fn jump_left_across_tree_different_branch_same_depth() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(**cursor, "LL");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Left).unwrap());
        assert_eq!(cursor.path(), &[0, 1, 1]);
        assert_eq!(**cursor, "RR");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Jump from L to R
    fn jump_left_across_tree_same_branch_same_depth() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(**cursor, "L");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Left).unwrap());
        assert_eq!(cursor.path(), &[0, 1]);
        assert_eq!(**cursor, "R");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_2`]
    ///
    /// Jump from LLL to LR
    fn jump_left_across_tree_to_shorter_branch() {
        let tree = binary_tree(BINARY_2);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0, 0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0, 0]);
        assert_eq!(**cursor, "LLL");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Left).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 1]);
        assert_eq!(**cursor, "LR");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_1`]
    ///
    /// Jump from LL to LR
    fn jump_left_across_tree_to_longer_branch() {
        let tree = binary_tree(BINARY_1);
        let mut cursor = tree.cursor().unwrap();

        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(**cursor, "LL");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Left).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 1]);
        assert_eq!(**cursor, "LR");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Jump from R to L
    fn jump_left_in_branch_same_depth() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 1]).unwrap());
        assert_eq!(cursor.path(), &[0, 1]);
        assert_eq!(**cursor, "R");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Left).unwrap());
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(**cursor, "L");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_1`]
    ///
    /// Jump from LRL to LL
    fn jump_left_between_branches_to_shorter_branch() {
        let tree = binary_tree(BINARY_1);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0, 1, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 1, 0]);
        assert_eq!(**cursor, "LRL");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Left).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(**cursor, "LL");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_3`]
    ///
    /// Jump from LRL to LLL
    fn jump_left_between_branches_to_longer_branch() {
        let tree = binary_tree(BINARY_3);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0, 1, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 1, 0]);
        assert_eq!(**cursor, "LRL");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Left).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0, 0]);
        assert_eq!(**cursor, "LLL");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Jump from RL to LR
    fn jump_left_between_branches_same_depth() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 1, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 1, 0]);
        assert_eq!(**cursor, "RL");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Left).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 1]);
        assert_eq!(**cursor, "LR");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`LIST`]
    fn jump_left_in_list() {
        let tree = binary_tree(LIST);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(**cursor, "LL");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Left).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(**cursor, "LL");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`EMPTY`]
    fn jump_left_in_empty_tree() {
        let tree = binary_tree(EMPTY);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Left).unwrap());
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Jump from RR to LL
    fn jump_right_across_tree_different_branch_same_depth() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 1, 1]).unwrap());
        assert_eq!(cursor.path(), &[0, 1, 1]);
        assert_eq!(**cursor, "RR");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Right).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(**cursor, "LL");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Jump from R to L
    fn jump_right_across_tree_same_branch_same_depth() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 1]).unwrap());
        assert_eq!(cursor.path(), &[0, 1]);
        assert_eq!(**cursor, "R");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Right).unwrap());
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(**cursor, "L");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_1`]
    ///
    /// Jump from LRL to LL
    fn jump_right_across_tree_to_shorter_branch() {
        let tree = binary_tree(BINARY_1);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0, 1, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 1, 0]);
        assert_eq!(**cursor, "LRL");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Right).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(**cursor, "LL");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_2`]
    ///
    /// Jump from LR to LL
    fn jump_right_across_tree_to_longer_branch() {
        let tree = binary_tree(BINARY_2);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0, 1]).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 1]);
        assert_eq!(**cursor, "LR");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Right).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(**cursor, "LL");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Jump from L to R
    fn jump_right_in_branch_same_depth() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(**cursor, "L");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Right).unwrap());
        assert_eq!(cursor.path(), &[0, 1]);
        assert_eq!(**cursor, "R");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_2`]
    ///
    /// Jump from LLL to LR
    fn jump_right_between_branches_to_shorter_branch() {
        let tree = binary_tree(BINARY_2);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0, 0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0, 0]);
        assert_eq!(**cursor, "LLL");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Right).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 1]);
        assert_eq!(**cursor, "LR");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_4`]
    ///
    /// Jump from LLL to LRL
    fn jump_right_between_branches_to_longer_branch() {
        let tree = binary_tree(BINARY_4);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0, 0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0, 0]);
        assert_eq!(**cursor, "LLL");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Right).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 1, 0]);
        assert_eq!(**cursor, "LRL");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Jump from LR to RL
    fn jump_right_between_branches_same_depth() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0, 1]).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 1]);
        assert_eq!(**cursor, "LR");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Right).unwrap());
        assert_eq!(cursor.path(), &[0, 1, 0]);
        assert_eq!(**cursor, "RL");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`LIST`]
    fn jump_right_in_list() {
        let tree = binary_tree(LIST);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(**cursor, "LL");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Right).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(**cursor, "LL");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`EMPTY`]
    fn jump_right_in_empty_tree() {
        let tree = binary_tree(EMPTY);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.jump(Direction::Right).unwrap());
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Attempt to walk from LL to RR
    fn walk_left_across_tree_different_branch_same_depth() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(**cursor, "LL");
        assert_children!(cursor.node().children(), &[]);

        assert!(!cursor.walk(Direction::Left).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(**cursor, "LL");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Attempt to walk from L to R
    fn walk_left_across_tree_same_branch_same_depth() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(**cursor, "L");
        assert_children!(cursor.node().children(), &[]);

        assert!(!cursor.walk(Direction::Left).unwrap());
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(**cursor, "L");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_2`]
    ///
    /// Attempt to walk from LLL to LR
    fn walk_left_across_tree_to_shorter_branch() {
        let tree = binary_tree(BINARY_2);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0, 0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0, 0]);
        assert_eq!(**cursor, "LLL");
        assert_children!(cursor.node().children(), &[]);

        assert!(!cursor.walk(Direction::Left).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0, 0]);
        assert_eq!(**cursor, "LLL");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_1`]
    ///
    /// Attempt to walk from LL to LR
    fn walk_left_across_tree_to_longer_branch() {
        let tree = binary_tree(BINARY_1);
        let mut cursor = tree.cursor().unwrap();

        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(**cursor, "LL");
        assert_children!(cursor.node().children(), &[]);

        assert!(!cursor.walk(Direction::Left).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(**cursor, "LL");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Move from R to L
    fn walk_left_in_branch_same_depth() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 1]).unwrap());
        assert_eq!(cursor.path(), &[0, 1]);
        assert_eq!(**cursor, "R");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.walk(Direction::Left).unwrap());
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(**cursor, "L");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_1`]
    ///
    /// Attempt to walk from LRL to LL
    fn walk_left_between_branches_to_shorter_branch() {
        let tree = binary_tree(BINARY_1);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0, 1, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 1, 0]);
        assert_eq!(**cursor, "LRL");
        assert_children!(cursor.node().children(), &[]);

        assert!(!cursor.walk(Direction::Left).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 1, 0]);
        assert_eq!(**cursor, "LRL");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_3`]
    ///
    /// Attempt to walk from LRL to LLL
    fn walk_left_between_branches_to_longer_branch() {
        let tree = binary_tree(BINARY_3);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0, 1, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 1, 0]);
        assert_eq!(**cursor, "LRL");
        assert_children!(cursor.node().children(), &[]);

        assert!(!cursor.walk(Direction::Left).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 1, 0]);
        assert_eq!(**cursor, "LRL");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Attempt to walk from RL to LR
    fn walk_left_between_branches_same_depth() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 1, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 1, 0]);
        assert_eq!(**cursor, "RL");
        assert_children!(cursor.node().children(), &[]);

        assert!(!cursor.walk(Direction::Left).unwrap());
        assert_eq!(cursor.path(), &[0, 1, 0]);
        assert_eq!(**cursor, "RL");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`LIST`]
    fn walk_left_in_list() {
        let tree = binary_tree(LIST);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(**cursor, "LL");
        assert_children!(cursor.node().children(), &[]);

        assert!(!cursor.walk(Direction::Left).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(**cursor, "LL");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`EMPTY`]
    fn walk_left_in_empty_tree() {
        let tree = binary_tree(EMPTY);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(!cursor.walk(Direction::Left).unwrap());
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Attempt to walk from RR to LL
    fn walk_right_across_tree_different_branch_same_depth() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 1, 1]).unwrap());
        assert_eq!(cursor.path(), &[0, 1, 1]);
        assert_eq!(**cursor, "RR");
        assert_children!(cursor.node().children(), &[]);

        assert!(!cursor.walk(Direction::Right).unwrap());
        assert_eq!(cursor.path(), &[0, 1, 1]);
        assert_eq!(**cursor, "RR");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Attempt to walk from R to L
    fn walk_right_across_tree_same_branch_same_depth() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 1]).unwrap());
        assert_eq!(cursor.path(), &[0, 1]);
        assert_eq!(**cursor, "R");
        assert_children!(cursor.node().children(), &[]);

        assert!(!cursor.walk(Direction::Right).unwrap());
        assert_eq!(cursor.path(), &[0, 1]);
        assert_eq!(**cursor, "R");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_1`]
    ///
    /// Attempt to walk from LRL to LL
    fn walk_right_across_tree_to_shorter_branch() {
        let tree = binary_tree(BINARY_1);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0, 1, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 1, 0]);
        assert_eq!(**cursor, "LRL");
        assert_children!(cursor.node().children(), &[]);

        assert!(!cursor.walk(Direction::Right).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 1, 0]);
        assert_eq!(**cursor, "LRL");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_2`]
    ///
    /// Attempt to walk from LR to LL
    fn walk_right_across_tree_to_longer_branch() {
        let tree = binary_tree(BINARY_2);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0, 1]).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 1]);
        assert_eq!(**cursor, "LR");
        assert_children!(cursor.node().children(), &[]);

        assert!(!cursor.walk(Direction::Right).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 1]);
        assert_eq!(**cursor, "LR");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Move from L to R
    fn walk_right_in_branch_same_depth() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(**cursor, "L");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.walk(Direction::Right).unwrap());
        assert_eq!(cursor.path(), &[0, 1]);
        assert_eq!(**cursor, "R");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_2`]
    ///
    /// Attempt to walk from LLL to LR
    fn walk_right_between_branches_to_shorter_branch() {
        let tree = binary_tree(BINARY_2);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0, 0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0, 0]);
        assert_eq!(**cursor, "LLL");
        assert_children!(cursor.node().children(), &[]);

        assert!(!cursor.walk(Direction::Right).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0, 0]);
        assert_eq!(**cursor, "LLL");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_4`]
    ///
    /// Attempt to walk from LLL to LRL
    fn walk_right_between_branches_to_longer_branch() {
        let tree = binary_tree(BINARY_4);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0, 0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0, 0]);
        assert_eq!(**cursor, "LLL");
        assert_children!(cursor.node().children(), &[]);

        assert!(!cursor.walk(Direction::Right).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0, 0]);
        assert_eq!(**cursor, "LLL");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_0`]
    ///
    /// Attempt to walk from LR to RL
    fn walk_right_between_branches_same_depth() {
        let tree = binary_tree(BINARY_0);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0, 1]).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 1]);
        assert_eq!(**cursor, "LR");
        assert_children!(cursor.node().children(), &[]);

        assert!(!cursor.walk(Direction::Right).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 1]);
        assert_eq!(**cursor, "LR");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`LIST`]
    fn walk_right_in_list() {
        let tree = binary_tree(LIST);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(cursor.go_to(&[0, 0, 0]).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(**cursor, "LL");
        assert_children!(cursor.node().children(), &[]);

        assert!(!cursor.walk(Direction::Right).unwrap());
        assert_eq!(cursor.path(), &[0, 0, 0]);
        assert_eq!(**cursor, "LL");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`EMPTY`]
    fn walk_right_in_empty_tree() {
        let tree = binary_tree(EMPTY);
        let mut cursor = tree.cursor().unwrap();
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);

        assert!(!cursor.walk(Direction::Right).unwrap());
        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_FULL`]
    fn cursor_drop() {
        let tree = binary_tree(BINARY_FULL);
        let mut cursor = tree.cursor().unwrap();

        assert_eq!(cursor.path(), &[0]);
        assert_eq!(**cursor, "ROOT");
        assert_children!(cursor.node().children(), &[]);
        assert_eq!(unsafe { *cursor.link.as_ref().in_use() }, 1);
        assert!(unsafe { cursor.link.as_ref().children().is_empty() });
        let root = Node::from_link(tree.root);
        assert_eq!(*root.in_use(), 1);
        assert!(root.children().is_empty());

        assert!(cursor.walk(Direction::Down(Target::First)).unwrap());
        assert_eq!(cursor.path(), &[0, 0]);
        assert_eq!(**cursor, "L");
        assert_children!(cursor.node().children(), &[]);
        assert_eq!(unsafe { *cursor.link.as_ref().in_use() }, 1);
        assert!(unsafe { cursor.link.as_ref().children().is_empty() });
        let root = Node::from_link(tree.root);
        assert_eq!(*root.in_use(), 1);
        assert!(!root.children().is_empty());

        drop(cursor);
        let root = Node::from_link(tree.root);
        assert_eq!(*root.in_use(), 0);
        assert!(root.children().is_empty());
    }

    #[test]
    #[serial]
    /// Uses [`BINARY_FULL`]
    fn multiple_cursors_one_thread() {
        let tree = binary_tree(BINARY_FULL);

        let mut cursor_0 = tree.cursor().unwrap();
        assert_eq!(cursor_0.path(), &[0]);
        assert_eq!(cursor_0.content, "ROOT");
        assert_children!(cursor_0.node().children(), &[]);
        let mut cursor_1 = tree.cursor().unwrap();
        assert_eq!(cursor_1.path(), &[0]);
        assert_eq!(cursor_1.content, "ROOT");
        assert_children!(cursor_1.node().children(), &[]);

        let mut cursor_2 = tree.cursor().unwrap();
        assert_eq!(cursor_2.path(), &[0]);
        assert_eq!(cursor_2.content, "ROOT");
        assert_children!(cursor_2.node().children(), &[]);
        let mut cursor_3 = tree.cursor().unwrap();
        assert_eq!(cursor_3.path(), &[0]);
        assert_eq!(cursor_3.content, "ROOT");
        assert_children!(cursor_3.node().children(), &[]);

        let mut cursor_4 = tree.cursor().unwrap();
        assert_eq!(cursor_4.path(), &[0]);
        assert_eq!(cursor_4.content, "ROOT");
        assert_children!(cursor_4.node().children(), &[]);
        let mut cursor_5 = tree.cursor().unwrap();
        assert_eq!(cursor_5.path(), &[0]);
        assert_eq!(cursor_5.content, "ROOT");
        assert_children!(cursor_5.node().children(), &[]);

        assert!(cursor_0.go_to(&[0, 0]).unwrap()); // ->
        assert_eq!(cursor_0.path(), &[0, 0]);
        assert_eq!(cursor_0.content, "L");
        assert_children!(cursor_0.node().children(), &[]);
        assert!(cursor_1.go_to(&[0, 1]).unwrap()); // <-
        assert_eq!(cursor_1.path(), &[0, 1]);
        assert_eq!(cursor_1.content, "R");
        assert_children!(cursor_1.node().children(), &[]);

        assert!(cursor_2.go_to(&[0, 0, 0]).unwrap()); // ->
        assert_eq!(cursor_2.path(), &[0, 0, 0]);
        assert_eq!(cursor_2.content, "LL");
        assert_children!(cursor_2.node().children(), &[]);
        assert!(cursor_3.go_to(&[0, 1, 1]).unwrap()); // <-
        assert_eq!(cursor_3.path(), &[0, 1, 1]);
        assert_eq!(cursor_3.content, "RR");
        assert_children!(cursor_3.node().children(), &[]);

        assert!(cursor_4.go_to(&[0, 0, 0, 0]).unwrap()); // ->
        assert_eq!(cursor_4.path(), &[0, 0, 0, 0]);
        assert_eq!(cursor_4.content, "LLL");
        assert_children!(cursor_4.node().children(), &[]);
        assert!(cursor_5.go_to(&[0, 1, 1, 1]).unwrap()); // <-
        assert_eq!(cursor_5.path(), &[0, 1, 1, 1]);
        assert_eq!(cursor_5.content, "RRR");
        assert_children!(cursor_5.node().children(), &[]);

        for _ in 0..10 {
            assert!(cursor_0.jump(Direction::Right).unwrap());
            assert!(cursor_0.jump(Direction::Right).unwrap());
            assert!(cursor_1.jump(Direction::Left).unwrap());
            assert!(cursor_1.jump(Direction::Left).unwrap());

            assert!(cursor_2.jump(Direction::Right).unwrap());
            assert!(cursor_2.jump(Direction::Right).unwrap());
            assert!(cursor_2.jump(Direction::Right).unwrap());
            assert!(cursor_2.jump(Direction::Right).unwrap());
            assert!(cursor_3.jump(Direction::Left).unwrap());
            assert!(cursor_3.jump(Direction::Left).unwrap());
            assert!(cursor_3.jump(Direction::Left).unwrap());
            assert!(cursor_3.jump(Direction::Left).unwrap());

            assert!(cursor_4.jump(Direction::Right).unwrap());
            assert!(cursor_4.jump(Direction::Right).unwrap());
            assert!(cursor_4.jump(Direction::Right).unwrap());
            assert!(cursor_4.jump(Direction::Right).unwrap());
            assert!(cursor_4.jump(Direction::Right).unwrap());
            assert!(cursor_4.jump(Direction::Right).unwrap());
            assert!(cursor_4.jump(Direction::Right).unwrap());
            assert!(cursor_4.jump(Direction::Right).unwrap());
            assert!(cursor_5.jump(Direction::Left).unwrap());
            assert!(cursor_5.jump(Direction::Left).unwrap());
            assert!(cursor_5.jump(Direction::Left).unwrap());
            assert!(cursor_5.jump(Direction::Left).unwrap());
            assert!(cursor_5.jump(Direction::Left).unwrap());
            assert!(cursor_5.jump(Direction::Left).unwrap());
            assert!(cursor_5.jump(Direction::Left).unwrap());
            assert!(cursor_5.jump(Direction::Left).unwrap());
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
                    let mut cursor_1 = tree.cursor().unwrap();

                    assert!(cursor_0.go_to(&[0, 0]).unwrap()); // ->
                    assert!(cursor_1.go_to(&[0, 1]).unwrap()); // <-

                    for _ in 0..40 {
                        assert!(cursor_0.jump(Direction::Right).unwrap());
                        assert!(cursor_0.jump(Direction::Right).unwrap());
                        assert!(cursor_1.jump(Direction::Left).unwrap());
                        assert!(cursor_1.jump(Direction::Left).unwrap());
                    }
                });

                s.spawn(|| {
                    let mut cursor_2 = tree.cursor().unwrap();
                    let mut cursor_3 = tree.cursor().unwrap();

                    assert!(cursor_2.go_to(&[0, 0, 0]).unwrap()); // ->
                    assert!(cursor_3.go_to(&[0, 1, 1]).unwrap()); // <-

                    for _ in 0..20 {
                        assert!(cursor_2.jump(Direction::Right).unwrap());
                        assert!(cursor_2.jump(Direction::Right).unwrap());
                        assert!(cursor_2.jump(Direction::Right).unwrap());
                        assert!(cursor_2.jump(Direction::Right).unwrap());
                        assert!(cursor_3.jump(Direction::Left).unwrap());
                        assert!(cursor_3.jump(Direction::Left).unwrap());
                        assert!(cursor_3.jump(Direction::Left).unwrap());
                        assert!(cursor_3.jump(Direction::Left).unwrap());
                    }
                });

                s.spawn(|| {
                    let mut cursor_4 = tree.cursor().unwrap();
                    let mut cursor_5 = tree.cursor().unwrap();

                    assert!(cursor_4.go_to(&[0, 0, 0, 0]).unwrap()); // ->
                    assert!(cursor_5.go_to(&[0, 1, 1, 1]).unwrap()); // <-

                    for _ in 0..10 {
                        assert!(cursor_4.jump(Direction::Right).unwrap());
                        assert!(cursor_4.jump(Direction::Right).unwrap());
                        assert!(cursor_4.jump(Direction::Right).unwrap());
                        assert!(cursor_4.jump(Direction::Right).unwrap());
                        assert!(cursor_4.jump(Direction::Right).unwrap());
                        assert!(cursor_4.jump(Direction::Right).unwrap());
                        assert!(cursor_4.jump(Direction::Right).unwrap());
                        assert!(cursor_4.jump(Direction::Right).unwrap());
                        assert!(cursor_5.jump(Direction::Left).unwrap());
                        assert!(cursor_5.jump(Direction::Left).unwrap());
                        assert!(cursor_5.jump(Direction::Left).unwrap());
                        assert!(cursor_5.jump(Direction::Left).unwrap());
                        assert!(cursor_5.jump(Direction::Left).unwrap());
                        assert!(cursor_5.jump(Direction::Left).unwrap());
                        assert!(cursor_5.jump(Direction::Left).unwrap());
                        assert!(cursor_5.jump(Direction::Left).unwrap());
                    }
                });
            }
        });
    }
}
