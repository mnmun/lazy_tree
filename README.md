# lazy_tree

Makes it possible to traverse a `tree` of lazily-created `nodes` using `cursors` without having to create the entire `tree` structure in advance. `Nodes` are created automatically using callbacks when the `cursor` needs to access them and are deleted when they are no longer used.

Used in [lazy_json](https://github.com/mnmun/lazy_json).

## `Tree`

![](https://github.com/mnmun/images/blob/main/tree.png?raw=true)

`Tree` is a wrapper aroung the `link` to the `root` `node` with the ability to create `cursors` for traversing hierarchical structure of lazily-created `nodes`. `Tree` may have multiple `cursors` at the same time, pointing to different `nodes` and moving independently of each other.

## `Cursor`

![](https://github.com/mnmun/images/blob/main/index_left.png?raw=true)

`Cursor` is a tool for traversing a `tree` of lazily-created `nodes`. It can move between `nodes` within the `tree` and provides access to the `value` stored in the `node` it currently points to. As it moves, the `cursor` triggers the creation of any `nodes` it need to access, as well as the deletion of `nodes` that are no longer in use.

## Callbacks

![](https://github.com/mnmun/images/blob/main/telephone.png?raw=true)

Each `node` in the `tree` contains two callbacks, which are invoked automatically by the `cursor` as it moves through the `tree`:

- `Total` - invoked on the `node` to which the `cursor` has just moved. It ensures the `query` callback functions correctly by calculating the `total` number of child `nodes` for a given parent `node`. If this calculation has already been performed for this `node`, it is not repeated;
- `Query` - invoked on a parent `node` when the `cursor` moves to its child `node` that has not yet been created. It handles the creation of a new child `node` only when access to that `node` is actually required.

When a `cursor` is created, the `total` callback is called on the `root` `node` as if the `cursor` had moved to it from the parent `node`.

In this way, a developer can define the logic for constructing a `tree` of lazily-created `nodes` tailored to their specific use case by implementing these callbacks.

## Example

![](https://github.com/mnmun/images/blob/main/bulb.png?raw=true)

The following example constructs a lazily-created binary `tree` from serialized data represented as `[ROOT, LEFT, RIGHT, LEFT-LEFT, LEFT-RIGHT, RIGHT-LEFT, RIGHT-RIGHT, ...]`:

```plain
                     ROOT                     
          ┌───────────┴───────────┐           
         LEFT                   RIGHT         
    ┌─────┴─────┐           ┌─────┴─────┐     
LEFT-LEFT   LEFT-RIGHT RIGHT-LEFT  RIGHT-RIGHT
   ...         ...         ...         ...
```

Let the `source` data be represented by a slice of elements of type `Option<&str>`, where missing `nodes` are denoted by `None`:

```rust
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
    /* [0]          */ Some("ROOT"),
    /* [0, 0]       */ Some("LEFT"),
    /* [0, 1]       */ Some("RIGHT"),
    /* [0, 0, 0]    */ Some("LEFT-LEFT"),
    /* [0, 0, 1]    */ Some("LEFT-RIGHT"),
    /* [0, 1, 0]    */ None, // RIGHT-LEFT
    /* [0, 1, 1]    */ Some("RIGHT-RIGHT"),
    /* [0, 0, 0, 0] */ Some("LEFT-LEFT-LEFT"),
    /* [0, 0, 0, 1] */ None, // LEFT-LEFT-RIGHT
];
```

The `value` stored in each `node` consists of the following fields:

- `content` - the `&str` associated with the `node`;
- `source` - a shared reference to the `source` data;
- `left_child_index` - the index of an element in the `source` data that is associated with the left child `node` of this parent `node`.

The `content` field is publicly accessible via a `cursor` using the `Deref` trait, while the `source` and `left_child_index` fields are internal states utilized only within the `total` and `query` callbacks:

```rust
// `Value` stored in each node
struct Bundle<'source> {
    // Publicly accessible via the `Deref` trait
    content: &'source str,

    // Internal state
    source: &'source[Option<&'source str>],
    left_child_index: usize,
}

// The `content` field can be accessed via cursor dereferencing: `**cursor`
impl<'source> Deref for Bundle<'source> {
    type Target = &'source str;

    fn deref(&self) -> &Self::Target {
        &self.content
    }
}

impl<'source> Bundle<'source> {
    // Method for creating a new bundle
    fn new(
        content: &'source str,
        source: &'source [Option<&'source str>],
        left_child_index: usize,
    ) -> Self {
        Self {
            content,
            source,
            left_child_index,
        }
    }
}
```

We need to specify a type to be used for errors returned by the `total` and `query` callbacks. Since there are no scenarios in this simple example where these callbacks might fail, this type will not be actually used; it is included here solely for demonstration purposes:

```rust
// The errors returned by the callbacks
#[derive(Debug)]
enum MyError {
    TotalError,
    QueryError,
};
```

The `total` callback determines the `total` number of child `nodes` that this parent `node` can have:

```rust
fn total<'source>(parent: &Bundle<'source>) -> Result<usize, MyError> {
    // The callback can return an error if necessary:
    // if condition {
    //     return Err(MyError::TotalError);
    // }

    let mut total = 0;

    if let Some(Some(_)) = parent.source.get(parent.left_child_index) {
        total += 1;
    }

    if let Some(Some(_)) = parent.source.get(parent.left_child_index + 1) {
        total += 1;
    }

    Ok(total)
}
```

The `query` callback creates a child `node` `builder` for the given `index` in the range `0..total`:

```rust
fn query<'source>(
    parent: &Bundle<'source>,
    index: usize,
) -> Result<node::Builder<Bundle<'source>, MyError>, MyError> {
    // The callback can return an error if necessary:
    // if condition {
    //     return Err(MyError::QueryError);
    // }

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
            parent.source,
            parent.left_child_index * 2 + 1,
        ),
        (None, Some(right)) if index == 0 => Bundle::new(
            right,
            parent.source,
            (parent.left_child_index + 1) * 2 + 1,
        ),
        (Some(_), Some(right)) if index == 1 => Bundle::new(
            right,
            parent.source,
            (parent.left_child_index + 1) * 2 + 1,
        ),
        _ => unreachable!(),
    };

    Ok(node::Builder::new(bundle, total, query))
}
```

Simple function for creating a `tree`, with validation of the `source` data:

```rust
fn make_tree<'source>(
    source: &'source [Option<&'source str>]
) -> Tree<Bundle<'source>, MyError> {
    if source.is_empty() {
        panic!("Source is empty");
    }

    if source[0].is_none() {
        panic!("Root is None");
    }

    Tree::new(node::Builder::new(
        Bundle::new(source[0].unwrap(), source, 1),
        total,
        query,
    ))
}
```

And finally, the binary `tree` can be traversed using the `cursor`:

```rust
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
    /* [0]          */ Some("ROOT"),
    /* [0, 0]       */ Some("LEFT"),
    /* [0, 1]       */ Some("RIGHT"),
    /* [0, 0, 0]    */ Some("LEFT-LEFT"),
    /* [0, 0, 1]    */ Some("LEFT-RIGHT"),
    /* [0, 1, 0]    */ None, // RIGHT-LEFT
    /* [0, 1, 1]    */ Some("RIGHT-RIGHT"),
    /* [0, 0, 0, 0] */ Some("LEFT-LEFT-LEFT"),
    /* [0, 0, 0, 1] */ None, // LEFT-LEFT-RIGHT
];

let tree = make_tree(source);

let mut cursor = tree.cursor().unwrap();
assert_eq!(cursor.path(), &[0]);
assert_eq!(**cursor, "ROOT");

cursor.walk(Direction::Down(Target::First));
assert_eq!(cursor.path(), &[0, 0]);
assert_eq!(**cursor, "LEFT");

cursor.walk(Direction::Right);
assert_eq!(cursor.path(), &[0, 1]);
assert_eq!(**cursor, "RIGHT");

cursor.walk(Direction::Down(Target::First));
assert_eq!(cursor.path(), &[0, 1, 0]);
assert_eq!(**cursor, "RIGHT-RIGHT");
```

The complete code:

```rust
use std::ops::Deref;
use pretty_assertions::assert_eq;

use lazy_tree::{
    Tree,
    node,
    cursor::{Direction, Target}
};

// `Value` stored in each node
struct Bundle<'source> {
    // Publicly accessible via the `Deref` trait
    content: &'source str,

    // Internal state
    source: &'source[Option<&'source str>],
    left_child_index: usize,
}

// The `content` field can be accessed via cursor dereferencing: `**cursor`
impl<'source> Deref for Bundle<'source> {
    type Target = &'source str;

    fn deref(&self) -> &Self::Target {
        &self.content
    }
}

impl<'source> Bundle<'source> {
    // Method for creating a new bundle
    fn new(
        content: &'source str,
        source: &'source [Option<&'source str>],
        left_child_index: usize,
    ) -> Self {
        Self {
            content,
            source,
            left_child_index,
        }
    }
}

// The errors returned by the callbacks
#[derive(Debug)]
enum MyError {
    TotalError,
    QueryError,
};

fn total<'source>(parent: &Bundle<'source>) -> Result<usize, MyError> {
    // The callback can return an error if necessary:
    // if condition {
    //     return Err(MyError::TotalError);
    // }

    let mut total = 0;

    if let Some(Some(_)) = parent.source.get(parent.left_child_index) {
        total += 1;
    }

    if let Some(Some(_)) = parent.source.get(parent.left_child_index + 1) {
        total += 1;
    }

    Ok(total)
}

fn query<'source>(
    parent: &Bundle<'source>,
    index: usize,
) -> Result<node::Builder<Bundle<'source>, MyError>, MyError> {
    // if condition {
    //     return Err(MyError::QueryError);
    // }

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
            parent.source,
            parent.left_child_index * 2 + 1,
        ),
        (None, Some(right)) if index == 0 => Bundle::new(
            right,
            parent.source,
            (parent.left_child_index + 1) * 2 + 1,
        ),
        (Some(_), Some(right)) if index == 1 => Bundle::new(
            right,
            parent.source,
            (parent.left_child_index + 1) * 2 + 1,
        ),
        _ => unreachable!(),
    };

    Ok(node::Builder::new(bundle, total, query))
}

fn make_tree<'source>(
    source: &'source [Option<&'source str>]
) -> Tree<Bundle<'source>, MyError> {
    if source.is_empty() {
        panic!("Source is empty");
    }

    if source[0].is_none() {
        panic!("Root is None");
    }

    Tree::new(node::Builder::new(
        Bundle::new(source[0].unwrap(), source, 1),
        total,
        query,
    ))
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
    /* [0]          */ Some("ROOT"),
    /* [0, 0]       */ Some("LEFT"),
    /* [0, 1]       */ Some("RIGHT"),
    /* [0, 0, 0]    */ Some("LEFT-LEFT"),
    /* [0, 0, 1]    */ Some("LEFT-RIGHT"),
    /* [0, 1, 0]    */ None, // RIGHT-LEFT
    /* [0, 1, 1]    */ Some("RIGHT-RIGHT"),
    /* [0, 0, 0, 0] */ Some("LEFT-LEFT-LEFT"),
    /* [0, 0, 0, 1] */ None, // LEFT-LEFT-RIGHT
];

let tree = make_tree(source);

let mut cursor = tree.cursor().unwrap();
assert_eq!(cursor.path(), &[0]);
assert_eq!(**cursor, "ROOT");

cursor.walk(Direction::Down(Target::First));
assert_eq!(cursor.path(), &[0, 0]);
assert_eq!(**cursor, "LEFT");

cursor.walk(Direction::Right);
assert_eq!(cursor.path(), &[0, 1]);
assert_eq!(**cursor, "RIGHT");

cursor.walk(Direction::Down(Target::First));
assert_eq!(cursor.path(), &[0, 1, 0]);
assert_eq!(**cursor, "RIGHT-RIGHT");
 ```

## License

[MIT](LICENSE)
