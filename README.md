![logo](https://github.com/mnmun/images/blob/main/tree.png?raw=true)

# A `tree` of lazily populated `nodes` with `cursor-based` traversal

This crate provides a `cursor-based` interface for traversing `trees` without materializing the entire hierarchy in memory. The `nodes` are populated lazily when visited by `cursors` and remain available only while they are needed by active `cursors`.

## Overview

Each `node` has an internal counter that tracks how many `cursors` are visiting it. When a `cursor` descends to a `node` (visits), its counter is incremented; when a `cursor` ascends from the `node` (leaves) - decremented. When a `cursor` visits a `node` whose current counter is zero, the `node's` `population callback` is invoked. The resulting children remain available until the last `cursor` leaves the `node`, at which point the counter reaches zero and children are released.

A `cursor` can move in both axes through the `tree`:

- Vertically, from a parent to one of its children and back;
- Horizontally, to the next or previous sibling, including siblings in an adjacent branch.

The following diagram shows the example `tree` in various states determined by the different positions of the `cursor`, indicated by asterisks:

![diagram_0](https://github.com/mnmun/lazy_tree/blob/main/diagrams/diagram_0.png?raw=true)

Consider a single `cursor` descending through the `tree` (`(0)` → `(3)`):

- `(0)`: The `tree` has no `cursors`, and only the `ROOT` `node` is present;
- `(1)`: A `cursor` is created at `ROOT`, and `ROOT's` children populated by a `callback`;
- `(2)`: The `cursor` descends from `ROOT` to `A` and visits `A`. Because this is the first `cursor` to visit `A`, its children are populated by the `callback`. `ROOT` and its children remain available;
- `(3)`: The `cursor` descends from `A` to `D` and visits `D`. Because this is the first `cursor` to visit `D`, its children are populated by the `callback`. `A` and its children remain available.

Thus, a `cursor` moving downward leaves behind a path of populated `nodes` from the `tree's` `root` to its current position.

Now consider the reverse traversal, in which the `cursor` ascends through the `tree` (`(3)` → `(0)`):

- `(3)`: The `cursor` is at `D`, whose children are currently populated;
- `(2)`: The `cursor` ascends from `D` to `A`; it leaves `D`, and its children are released because no other `cursor` keeps `D` visited;
- `(1)`: The `cursor` ascends from `A` to `ROOT`; it leaves `A`, and its children are released because no other `cursor` keeps `A` visited;
- `(0)`: The `cursor` is dropped, and all `nodes` except `ROOT` in the `tree` are released.

In other words, `nodes` populated during traversal are released automatically once no `cursor` keeps them visited.

## Types

The crate provides the following main types:

- `Tree` - owns the `root` `node` and creates `cursors` for traversal;
- `Cursor` - traverses and inspects the `tree`;
- `Node` - stores a `value`, a `link` to its parent and links to its currently populated children;
- `Populate` - a function used to create a `node's` children when a `cursor` visits a `node` that is not currently visited by another `cursor`.

## Generic parameters

Many types in this crate use the following generic parameters:

- `Value` - the type of the `value` stored in each `node` through `Cow<'source, Value>`;
- `Source` - the element type of the `source` collection (`Vec<Source>` or `&[Source]`) used to populate `nodes`. The `source` collection is stored in each `node` through `Cow<'source, [Source]>`;
- `Error` - the `error` type returned by the `population callback`.

## Safety

A `tree` can be shared between threads, and multiple `cursors` may traverse the same `tree` concurrently. Access to the `root` and to each `node's` traversal state is synchronized internally. Each `cursor` still borrows the `tree` for its entire lifetime and therefore cannot outlive the `tree` from which it was created.

## Example

The following example constructs a lazily populated binary `tree` from serialized data represented as `[ROOT, LEFT, RIGHT, LEFT-LEFT, LEFT-RIGHT, RIGHT-LEFT, RIGHT-RIGHT, ...]`:

![diagram_1](https://github.com/mnmun/lazy_tree/blob/main/diagrams/diagram_1.png?raw=true)

This example uses `&str` as the `node` `value` type and `Option<&str>` as the element type of the `source` collection. An absent value is represented by `None`, which allows the `source` collection to describe missing nodes.

The `population callback` creates a `node's` children from two consecutive elements of the `source` collection, starting at the index specified by the `node's` `range`. A `node` without a `range` is treated as a leaf.

```rust
use std::{borrow::Cow, ops::Range};
use pretty_assertions::assert_eq;

use tree::{Tree, Builder, Link, Direction, Target};

// The type of the value stored in each node
type MyValue = str;

// The type of an element in the source collection
type MySource<'source> = Option<&'source str>;

// The error returned by the population callback
#[derive(Debug)]
enum MyError {
   // Indicates that the source collection is empty
   SourceIsEmpty,
};

// A type alias for `Link` with the type parameters specified
type MyLink<'source> = Link<'source, MyValue, MySource<'source>, MyError>;

// A type alias for a boxed slice of `MyLink`
type Children<'source> = Box<[MyLink<'source>]>;

fn populate<'source>(
   source: impl Into<Cow<'source, [MySource<'source>]>>,
   range: Option<Range<usize>>,
   parent: MyLink<'source>,
) -> Result<Children<'source>, MyError> {
   let range = if let Some(range) = range {
       range
   } else {
       // A node without a range is a leaf
       return Ok(Box::default());
   };

   let source = source.into();

   if source.is_empty() {
       return Err(MyError::SourceIsEmpty);
   }

   // The range identifies the positions of the node's children in the
   // source collection
   let left_child = source.get(range.start).cloned();
   let right_child = source.get(range.start + 1).cloned();

   let mut children = vec![];

   if let Some(Some(value)) = left_child {
       children.push(
           Builder::new(
               Cow::Borrowed(value),
               source.clone(),
               populate, // Reuses the same callback for child nodes
           )
           .with_parent(parent) // Required for upward traversal
           .with_range({
               let start = range.start * 2 + 2;
               if source.len() < start {
                   None
               } else {
                   Some(start..source.len())
               }
           })
           .build()
       );
   }

   if let Some(Some(value)) = right_child {
       children.push(
           Builder::new(
               Cow::Borrowed(value),
               source.clone(),
               populate, // Reuses the same callback for child nodes
           )
           .with_parent(parent) // Required for upward traversal
           .with_range({
               let start = (range.start + 1) * 2 + 2;
               if source.len() < start {
                   None
               } else {
                   Some(start..source.len())
               }
           })
           .build()
       );
   }

   Ok(children.into_boxed_slice())
}

//                                    
//                    ROOT            
//            ┌────────┴────────┐     
//           LEFT             RIGHT   
//       ┌────┴────┐            │     
//   LEFT-LEFT LEFT-RIGHT  RIGHT-RIGHT
//       │                            
// LEFT-LEFT-LEFT                     
//                                    
let source: &[Option<&str>] = &[
   Some("LEFT"),
   Some("RIGHT"),
   Some("LEFT-LEFT"),
   Some("LEFT-RIGHT"),
   None, // RIGHT-LEFT
   Some("RIGHT-RIGHT"),
   Some("LEFT-LEFT-LEFT"),
   None, // LEFT-LEFT-RIGHT
];

let tree = Tree::new(
   Builder::new("ROOT", source.clone(), populate)
       .with_range(0..source.len())
       .build()
);

let mut cursor = tree.cursor().unwrap();
assert_eq!(cursor.path(), &[0]);
assert_eq!(cursor.value(), "ROOT");

cursor.walk(Direction::Down(Target::First));
assert_eq!(cursor.path(), &[0, 0]);
assert_eq!(cursor.value(), "LEFT");

cursor.walk(Direction::Right);
assert_eq!(cursor.path(), &[0, 1]);
assert_eq!(cursor.value(), "RIGHT");

cursor.walk(Direction::Down(Target::First));
assert_eq!(cursor.path(), &[0, 1, 0]);
assert_eq!(cursor.value(), "RIGHT-RIGHT");
```

[Here](https://github.com/mnmun/json) you could find a more complex example of a JSON parser build on top of this crate.

## Features

This crate provides a set of optional features that can be enabled in your `Cargo.toml` file:

- `debug` - Enables memory leak detection.
  Each created `node` increments a global atomic counter. Dropping a `node` decrements the counter. If the counter is not zero when the `tree` is dropped, it indicates that some `nodes` were not deallocated, signaling a memory leak with a panic.
  Note that `trees` share the same global `node` counter. If multiple `trees` exist simultaneously, dropping one `tree` while another has alive `nodes` will be flagged as a memory leak and cause a panic.
  Use this feature in sequentually executed tests where only one `tree` exists at a time!

## License

[MIT](LICENSE)
