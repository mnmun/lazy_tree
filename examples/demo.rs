#![allow(dead_code)]

use pretty_assertions::assert_eq;
use std::ops::Deref;

use lazy_tree::{
    Tree,
    cursor::{Direction, Target},
    node,
};

// `Value` stored in each node
struct Bundle<'source> {
    // Publicly accessible via the `Deref` trait
    content: &'source str,

    // Internal state
    source: &'source [Option<&'source str>],
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
}

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
        (Some(left), _) if index == 0 => {
            Bundle::new(left, parent.source, parent.left_child_index * 2 + 1)
        }
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
    source: &'source [Option<&'source str>],
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

fn main() {
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

    assert!(cursor.walk(Direction::Down(Target::First)).unwrap());
    assert_eq!(cursor.path(), &[0, 0]);
    assert_eq!(**cursor, "LEFT");

    assert!(cursor.walk(Direction::Right).unwrap());
    assert_eq!(cursor.path(), &[0, 1]);
    assert_eq!(**cursor, "RIGHT");

    assert!(cursor.walk(Direction::Down(Target::First)).unwrap());
    assert_eq!(cursor.path(), &[0, 1, 0]);
    assert_eq!(**cursor, "RIGHT-RIGHT");
}
