![](https://github.com/mnmun/images/blob/main/tree.png?raw=true)

# A `tree` of lazily populated `nodes` with `cursor-based` traversal

Provides a `cursor-based` interface for traversing `trees` of lazily populated `nodes` without materializing the entire hierarchy in memory. The `nodes` are populated only when visited by `cursors` and remain available only while they are needed.

Each `node` has an internal counter that tracks how many `cursors` are visiting it. When a `cursor` descends to (visits) a `node`, its counter is incremented; when a `cursor` ascends from (leaves) the `node` - decremented. When a `cursor` visits a `node` whose current counter is zero, the `node's` `callback` is invoked. The resulting children remain available until the last `cursor` leaves the `node`, at which point the counter reaches zero and children are released.

A `cursor` can move in both axes through the `tree`:

 - Vertically, from a parent to one of its children or in the opposite direction;
 - Horizontally, to the next or previous sibling, including siblings in an adjacent branch.

The following diagram shows the example `tree` in various states determined by the positions of the `cursor`, indicated by asterisks:

![](https://github.com/mnmun/lazy_tree/blob/main/diagrams/diagram_0.png?raw=true)

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

In other words, `nodes` `populated` during traversal and released automatically once no `cursor` keeps them visited.

## Types

![](https://github.com/mnmun/images/blob/main/handle_with_care.png?raw=true)

The crate provides the following main types:

 - `Tree` - owns the `root` `node` and creates `cursors`;
 - `Cursor` - traverses and inspects the `tree`;
 - `Node` - stores `value`, `link` to parent and `links` to currently populated children;
 - `Populate` - a function used to create a `node's` children when a `cursor` visits an unvisited `node`.

## Generic parameters

![](https://github.com/mnmun/images/blob/main/umbrella.png?raw=true)

The following generic parameters are used in the types of this crate:

 - `Value` - the type of the `value` stored in each `node`;
 - `Error` - the type returned by the `callback` in the event of failure.

## Safety

![](https://github.com/mnmun/images/blob/main/slippery.png?raw=true)

A `tree` can be shared between threads, and multiple `cursors` may traverse the same `tree` concurrently. Access to the `root` and to each `node's` traversal state is synchronized internally. Each `cursor` still borrows the `tree` for its entire lifetime and therefore cannot outlive the `tree` from which it was created.

## Example

![](https://github.com/mnmun/images/blob/main/bulb.png?raw=true)

The following example constructs a lazily populated binary `tree` from serialized data represented as `[ROOT, LEFT, RIGHT, LEFT-LEFT, LEFT-RIGHT, RIGHT-LEFT, RIGHT-RIGHT, ...]`:

![](https://github.com/mnmun/lazy_tree/blob/main/diagrams/diagram_1.png?raw=true)

Let the source data be represented by a slice of elements of type `Option<&str>`, where missing `nodes` are denoted by `None`.

The `value` stored in each `node` consists of the following fields:

 - `content` - a `&str` value associated with the `node`;
 - `source` - a reference to the source data;
 - `position` - an index within the source data used to determine the creation of child `nodes`.

The `content` field is publicly accessible via the `content()` method, while the `source` and `position` fields are internal states utilized only within the `populate` `callback`.

The `populate` `callback` creates two or fewer child `nodes` based on the `source` and the `position` of the parent `node`.

```rust
use std::{borrow::Cow, ops::Range};
use pretty_assertions::assert_eq;

use lazy_tree::{
  Tree,
  Node,
  node::Link,
  cursor::{Direction, Target}
};

// `Value` stored in each node
struct Bundle<'source> {
    // Publicly accessible via the `content()` method
    content: &'source str,

    // Internal state
    source: &'source[Option<&'source str>],
    position: usize,
}

impl<'source> Bundle<'source> {
    pub fn content(&self) -> &'source str {
        &self.content
    }
}

// The error returned by the callback
#[derive(Debug)]
enum MyError {
    // Indicates that the source is empty
    SourceIsEmpty,
};

// A type alias for `Link` with the type parameters specified
type MyLink<'source> = Link<Bundle<'source>, MyError>;

// A type alias for a boxed slice of `MyLink`
type Children<'source> = Box<[MyLink<'source>]>;

fn populate<'source>(
    parent: &Bundle<'source>,
) -> Result<Children<'source>, MyError> {
    if parent.source.is_empty() {
        return Err(MyError::SourceIsEmpty);
    }

    if parent.position >= parent.source.len() {
        return Ok(Box::default());
    }

    let left_child = parent.source.get(parent.position).cloned();
    let right_child = parent.source.get(parent.position + 1).cloned();

    let mut children = vec![];

    if let Some(Some(value)) = left_child {
        let bundle = Bundle {
            content: value,
            source: parent.source,
            position: parent.position * 2 + 2,
        };

        children.push(Node::new(bundle, populate));
    }

    if let Some(Some(value)) = right_child {
        let bundle = Bundle {
            content: value,
            source: parent.source,
            position: (parent.position + 1) * 2 + 2,
        };

        children.push(Node::new(bundle, populate));
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
    Some("ROOT"),
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
    Node::new(
        Bundle {
            content: source[0].unwrap(),
            source: &source[1..],
            position: 0
        },
        populate
    )
);

let mut cursor = tree.cursor().unwrap();
assert_eq!(cursor.path(), &[0]);
assert_eq!(cursor.value().content(), "ROOT");

cursor.walk(Direction::Down(Target::First));
assert_eq!(cursor.path(), &[0, 0]);
assert_eq!(cursor.value().content(), "LEFT");

cursor.walk(Direction::Right);
assert_eq!(cursor.path(), &[0, 1]);
assert_eq!(cursor.value().content(), "RIGHT");

cursor.walk(Direction::Down(Target::First));
assert_eq!(cursor.path(), &[0, 1, 0]);
assert_eq!(cursor.value().content(), "RIGHT-RIGHT");
```

The [`lazy_json`](https://github.com/mnmun/lazy_json) crate includes a JSON parser built using this crate, which can serve as an usage example.

## Features

![](https://github.com/mnmun/images/blob/main/pot.png?raw=true)

This crate provides a set of optional features that can be enabled in `Cargo.toml` file:

- `debug` - Enables memory leak detection.
  Each created `node` increments a global atomic counter. Dropping a `node` decrements the counter. If the counter is not zero when the `tree` is dropped, it indicates that some `nodes` were not deallocated, signaling a memory leak with a panic.
  Note that `trees` share the same global `node` counter. If multiple `trees` exist simultaneously, dropping one `tree` while another has alive `nodes` will be flagged as a memory leak and cause a panic.
  Use this feature in sequentually executed tests where only one `tree` exists at a time!

## License

[MIT](LICENSE)
