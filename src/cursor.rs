//! # Traversing a [`tree`] with a [`cursor`]
//!
//! This module defines a [`cursor`] - the tool for traversing a [`tree`] of
//! lazily populated [`nodes`].
//!
//! See [`crate`] for more information.
//!
//! [`tree`]: crate::Tree
//! [`cursor`]: Cursor
//! [`nodes`]: Node

use std::{marker::PhantomData, ops::Deref};

use getset::Getters;

use crate::{Link, Node};

/// Specifies which child [`node`] [`cursor`] selects when moving down
///
/// [`node`]: Node
/// [`cursor`]: Cursor
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Target {
    /// Selects the child at the specified zero-based index
    Index(usize),
    #[default]
    First,
    Last,
}

/// # Specifies the direction of a [`cursor`] movement
///
/// See [`walk()`] and [`jump()`] for more information.
///
/// [`cursor`]: Cursor
/// [`walk()`]: Cursor::walk()
/// [`jump()`]: Cursor::jump()
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Direction {
    /// Moves the [`cursor`] to the parent [`node`]
    ///
    /// [`cursor`]: Cursor
    /// [`node`]: Node
    Up,

    /// Moves the [`cursor`] to a child [`node`] selected by [`target`]
    ///
    /// [`cursor`]: Cursor
    /// [`node`]: Node
    /// [`target`]: Target
    Down(Target),

    /// Moves the [`cursor`] to the previous sibling [`node`]
    ///
    /// [`cursor`]: Cursor
    /// [`node`]: Node
    Left,

    /// Moves the [`cursor`] to the next sibling [`node`]
    ///
    /// [`cursor`]: Cursor
    /// [`node`]: Node
    Right,
}

/// ![cursor](https://github.com/mnmun/images/blob/main/index_left.png?raw=true)
///
/// # `Cursor`
///
/// `Cursor` stores a [`link`] to the [`node`] it currently visits and the
/// absolute [`path`] from the [`tree's`] `root` to that [`node`].
///
/// `Cursor` dereferences to a [`&Node`], allowing direct access to the
/// [`node's`] public data.
///
/// [`Path`] contains one zero-based child index for each passed level in the
/// [`tree`].
///
/// The following diagram shows the example [`tree`] in various states
/// determined by the different positions of the `cursor`, indicated by
/// asterisks:
///
/// ```plain
///                                 ROOT
///                  ROOT         ┌──┼──┐
///    *ROOT*      ┌──┼──┐        A  B  C
///   ┌──┼──┐      A *B* C        ┌──┼──┐
///   A  B  C      ┌──┼──┐       *D* E  F
///                D  E  F     ┌──┼──┐   
///     (0)                    G  H  I   
///                  (1)                 
///                                (2)   
///
/// ┌──────┬─────────────────┬──────────────────┐
/// │ Case │ Cursor position │ Cursor path      │
/// ├──────┼─────────────────┼──────────────────┤
/// │ (0)  │ ROOT            │ [0]              │
/// ├──────┼─────────────────┼──────────────────┤
/// │ (1)  │ B               │ [0, 1]           │
/// ├──────┼─────────────────┼──────────────────┤
/// │ (2)  │ D               │ [0, 1, 0]        │
/// └──────┴─────────────────┴──────────────────┘
/// ```
///
/// Thus, all valid [`paths`] could not be empty and should start with zero.
///
/// ## Creation
///
/// Use [`Tree::cursor()`] to create a new `cursor`. The `cursor` initially
/// points to the [`tree's`] `root` and has the [`path`] set to `[0]`.
///
/// [`Tree`] may have multiple `cursors` at the same time. `Cursors`
/// implement [`Send`], so they can be transferred between threads safely. Each
/// `cursor` remains borrowed from its [`tree`] and cannot outlive it.
///
/// ## Tree traversing
///
/// `Cursor` has several methods for moving between [`nodes`] in the [`tree`]:
///
/// - [`go_to()`] - moves `cursor` to a [`node`] by a provided absolute `path`;
/// - [`walk()`] - performs at most one edge traversal in provided
///   [`direction`];
/// - [`jump()`] - searches across parent, child and sibling branches when the
///   adjacent move in provided [`direction`] is unavailable.
///
/// See [`crate`] for more information.
///
/// [`link`]: Link
/// [`node`]: Node
/// [`node's`]: Node
/// [`nodes`]: Node
/// [`&Node`]: Node
/// [`path`]: Cursor::path()
/// [`paths`]: Cursor::path()
/// [`Path`]: Cursor::path()
/// [`tree`]: crate::Tree
/// [`tree's`]: crate::Tree
/// [`Tree`]: crate::Tree
/// [`Tree::cursor()`]: crate::Tree::cursor()
/// [`go_to()`]: Cursor::go_to()
/// [`walk()`]: Cursor::walk()
/// [`jump()`]: Cursor::jump()
/// [`direction`]: Direction
#[derive(Debug, Getters)]
pub struct Cursor<'tree, 'source, Value, Source, Error>
where
    Value: ToOwned + ?Sized + 'source,
    Source: Clone + 'source,
{
    /// [`Link`] to the [`node`] being currently observed
    ///
    /// [`node`]: Node
    pub(crate) link: Link<'source, Value, Source, Error>,

    /// # Absolute `path` from the [`tree's`] `root` to the [`node`] being currently observed
    ///
    /// Every subsequent element is the zero-based index of the selected child
    /// at that [`tree`] level.
    ///
    /// The `root` is located at the path `[0]`.
    ///
    /// [`tree`]: crate::Tree
    /// [`tree's`]: crate::Tree
    /// [`node`]: Node
    #[getset(get = "pub")]
    path: Vec<usize>,

    /// # Important!
    ///
    /// This marker prevents the `cursor` from outliving the [`tree`] from
    /// which it was created.
    ///
    /// [`tree`]: crate::Tree
    _marker: PhantomData<&'tree ()>,
}

/// # Safety
///
/// [`Cursor`] contains a raw [`non-null`] pointer in a form of a [`link`]. It
/// is safe to send a [`cursor`] to another thread because mutable access is
/// synchronized by the [`node's`] `visitors` `mutex`, and the [`cursor`] is
/// constrained by the lifetime of its [`tree`].
///
/// [`non-null`]: std::ptr::NonNull
/// [`link`]: Link
/// [`cursor`]: Cursor
/// [`node's`]: Node
/// [`tree`]: crate::Tree
unsafe impl<'tree, 'source, Value, Source, Error> Send
    for Cursor<'tree, 'source, Value, Source, Error>
where
    Value: ToOwned + ?Sized + 'source,
    Source: Clone + 'source,
{
}

impl<'tree, 'source, Value, Source, Error> Drop
    for Cursor<'tree, 'source, Value, Source, Error>
where
    Value: ToOwned + ?Sized + 'source,
    Source: Clone + 'source,
{
    /// # Releases all [`node`] visits held by this [`cursor`]
    ///
    /// The [`cursor`] first returns to the `root`, releasing every
    /// intermediate [`node`], and then releases its final visit to the `root`.
    ///
    /// [`node`]: Node
    /// [`cursor`]: Cursor
    fn drop(&mut self) {
        let _ = self.go_to(&[0]); // `go_to()` could not fail in upward movement
        self.leave();
    }
}

impl<'tree, 'source, Value, Source, Error> Deref
    for Cursor<'tree, 'source, Value, Source, Error>
where
    Value: ToOwned + ?Sized + 'source,
    Source: Clone + 'source,
{
    type Target = Node<'source, Value, Source, Error>;

    fn deref(&self) -> &Self::Target {
        unsafe { self.link.as_ref() }
    }
}

/// # Returns the first index at which two slices differ
///
/// If one slice is a non-empty prefix of the other, returns `None`.
/// If exactly one slice is empty, returns `Some(0)`.
fn vec_equ<T: PartialEq>(a: &[T], b: &[T]) -> Option<usize> {
    if (a.is_empty() && !b.is_empty()) || (!a.is_empty() && b.is_empty()) {
        return Some(0);
    }

    a.iter()
        .zip(b.iter())
        .enumerate()
        .find_map(|(i, (x, y))| if x != y { Some(i) } else { None })
}

impl<'tree, 'source, Value, Source, Error>
    Cursor<'tree, 'source, Value, Source, Error>
where
    Value: ToOwned + ?Sized + 'source,
    Source: Clone + 'source,
{
    /// # Creates a [`cursor`] pointed to the [`link`] with the given `path`
    ///
    /// The [`cursor`] is registered as a visitor of the [`node`]. If it the
    /// first visitor, the [`node's`] children are [`populated`] before the
    /// [`cursor`] is returned.
    ///
    /// # Errors
    ///
    /// Returns the [`population`] error unchanged. In that case, no [`cursor`]
    /// is produced.
    ///
    /// See [`crate`] for more information.
    ///
    /// [`cursor`]: Cursor
    /// [`link`]: Link
    /// [`node`]: Node
    /// [`node's`]: Node
    /// [`populated`]: crate::Populate
    /// [`population`]: crate::Populate
    pub(crate) fn new(
        link: Link<'source, Value, Source, Error>,
        path: Vec<usize>,
    ) -> Result<Self, Error> {
        unsafe {
            link.as_ref().visit(link)?;
        }
        Ok(Cursor {
            link,
            path,
            _marker: PhantomData,
        })
    }

    /// # Moves the [`cursor`] to the [`node`] addressed by an absolute `path`
    ///
    /// The [`cursor`] follows the longest valid prefix of `path`. If no valid
    /// prefix exist, the [`cursor`] returns to the [`tree's`] `root` (`[0]`).
    ///
    /// An empty `path` is considered invalid and leaves the [`cursor`]
    /// unchanged.
    ///
    /// # Returns
    ///
    /// - `Ok(true)` - if the complete `path` exists and was selected;
    /// - `Ok(false)` - if the `path` is invalid;
    /// - `Err(error)` - if [`populating`] a [`node's`] children fails. Movement
    ///   performed before the error is retained.
    ///
    /// See [`crate`] for more information.
    ///
    /// [`cursor`]: Cursor
    /// [`node`]: Node
    /// [`node's`]: Node
    /// [`tree's`]: crate::Tree
    /// [`populating`]: crate::Populate
    pub fn go_to(&mut self, path: &[usize]) -> Result<bool, Error> {
        if path.is_empty() {
            return Ok(false);
        }

        // If first step in path not `0` simply go to `[0]` and consider it
        // wrong path with `Ok(false)`
        if path.first().is_some_and(|first| *first != 0) {
            while self.path.len() != 1 {
                self.up();
            }
            return Ok(false);
        }

        if self.path == path {
            return Ok(true);
        }

        let mut result = true;

        if let Some(i) = vec_equ(&self.path, path) {
            // Because above we handled the situation when `i` could be `0`
            while self.path.len() != i {
                self.up();
            }
        }

        if self.path.len() < path.len() {
            while self.path.len() != path.len() {
                result &= self.down(Target::Index(path[self.path.len()]))?;

                if !result {
                    break;
                }
            }
        } else {
            while self.path.len() != path.len() {
                self.up();
            }
        }

        Ok(result)
    }

    /// # Attempts to perform exactly one movement in the given [`direction`]
    ///
    /// This function crosses at most one edge of the [`tree`]:
    ///
    /// ```plain
    /// ┌───────────┬──────────────────────────────────────────┐
    /// │ Direction │ Moves cursor to the                      │
    /// ├───────────┼──────────────────────────────────────────┤
    /// │ Up        │ Parent node                              │
    /// ├───────────┼──────────────────────────────────────────┤
    /// │ Down      │ Child node selected by target            │
    /// ├───────────┼──────────────────────────────────────────┤
    /// │ Right     │ Next sibling node in the same branch     │
    /// ├───────────┼──────────────────────────────────────────┤
    /// │ Left      │ Previous sibling node in the same branch │
    /// └───────────┴──────────────────────────────────────────┘
    /// ```
    ///
    /// If the requested adjacent [`node`] does not exist, the [`cursor`]
    /// remains at its current position and the attempt to move is considered
    /// failed. Otherwise the [`cursor`] successfully moves to the target
    /// [`node`].
    ///
    /// Returns the success of attempt or an `error` if [`populating`] a target
    /// [`node`] fails.
    ///
    /// # Examples
    ///
    /// Consider an example [`tree`] where the [`cursor`] initially points to
    /// `B`:
    ///
    /// ```plain
    ///             ROOT
    ///      ┌───────┼───────┐
    ///      A      *B*      C
    ///   ┌──┼──┐ ┌──┼──┐ ┌──┼──┐
    ///   D  E  F G  H  I J  K  L
    ///
    ///             (0)
    /// ```
    ///
    /// The nodes `D`, `E`, `F`, `J`, `K` and `L` do not exist yet: children
    /// are populated lazily when their parent is visited by [`cursor`]. They
    /// are shown only to illustrate the complete logical shape of the [`tree`].
    ///
    /// The following calls succeed:
    ///
    /// ```plain
    ///   *ROOT*        ROOT          ROOT                ROOT  
    ///   ┌─┼─┐      ┌───┼───┐     ┌───┼───┐           ┌───┼───┐
    ///   A B C      A   B   C     A   B  *C*         *A*  B   C
    ///               ┌──┼──┐           ┌──┼──┐     ┌──┼──┐     
    ///              *G* H  I           J  K  L     D  E  F     
    ///                                                         
    ///    (1)          (2)            (3)               (4)    
    /// ```
    ///
    /// - `(1)` = `(0)` + `walk(Direction::Up)`
    /// - `(2)` = `(0)` + `walk(Direction::Down(Target::First))`
    /// - `(3)` = `(0)` + `walk(Direction::Right)`
    /// - `(4)` = `(0)` + `walk(Direction::Left)`
    ///
    /// Keep in mind that horizontal movements are internally implemented as a
    /// sequence of vertical movements. They can therefore be expressed as
    /// follows:
    ///
    /// - `(3)` = `(0)` + `walk(Direction::Up)` + `walk(Direction::Down(Target::Last))`
    /// - `(4)` = `(0)` + `walk(Direction::Up)` + `walk(Direction::Down(Target::First))`
    ///
    /// In the following cases, the requested adjacent [`node`] does not exist.
    /// The [`cursor`] therefore remains in place and `walk()` returns
    /// `false`:
    ///
    /// - `(1)` + `walk(Direction::Up)` (`ROOT` has no parent)
    /// - `(2)` + `walk(Direction::Down(...))` (`G` is a leaf)
    /// - `(3)` + `walk(Direction::Right)` (`C` is the last sibling)
    /// - `(4)` + `walk(Direction::Left)` (`A` is the first sibling)
    ///
    /// [`cursor`]: Cursor
    /// [`node`]: Node
    /// [`direction`]: Direction
    /// [`target`]: Target
    /// [`tree`]: crate::Tree
    /// [`populating`]: crate::Populate
    pub fn walk(&mut self, direction: Direction) -> Result<bool, Error> {
        match direction {
            Direction::Up => Ok(self.up()),
            Direction::Down(target) => self.down(target),
            Direction::Left => self.left(),
            Direction::Right => self.right(),
        }
    }

    /// # Performs a directional search that may cross multiple [`tree`] edges
    ///
    /// This function behaves like [`walk()`] when the requested adjacent
    /// [`node`] exists. If [`walk()`] cannot make the requested move, `jump()`
    /// continues searching in the same [`direction`] and moves the [`cursor`]
    /// according to the rules described below:
    ///
    /// ```plain
    /// ┌───────────┬─────────────────────────────────────────────────────────┐
    /// │ Direction │ Moves cursor to the                                     │
    /// ├───────────┼─────────────────────────────────────────────────────────┤
    /// │ Up        │ Deepest leaf node in the first-child based path         │
    /// ├───────────┼─────────────────────────────────────────────────────────┤
    /// │ Down      │ Tree's root                                             │
    /// ├───────────┼─────────────────────────────────────────────────────────┤
    /// │ Right     │ Node of the next branch got by descending through first │
    /// │           │ children                                                │
    /// ├───────────┼─────────────────────────────────────────────────────────┤
    /// │ Left      │ Node of the previous branch got by descending through   │
    /// │           │ last children                                           │
    /// └───────────┴─────────────────────────────────────────────────────────┘
    /// ```
    ///
    /// When no branch exists in the requested horizontal [`direction`], the
    /// search wraps around the [`tree`].
    ///
    /// Always return `true` or an `error` if [`populating`] a [`node`] fails.
    ///
    /// # Examples
    ///
    /// The four configurations below are the successful results from the
    /// corresponding [`walk()`] calls in its example:
    ///
    /// ```plain
    ///   *ROOT*        ROOT          ROOT                ROOT  
    ///   ┌─┼─┐      ┌───┼───┐     ┌───┼───┐           ┌───┼───┐
    ///   A B C      A   B   C     A   B  *C*         *A*  B   C
    ///               ┌──┼──┐           ┌──┼──┐     ┌──┼──┐     
    ///              *G* H  I           J  K  L     D  E  F     
    ///                                                         
    ///    (1)          (2)            (3)               (4)    
    /// ```
    ///
    /// Applying `jump()` to the cases where [`walk()`] could not move produces
    /// the following states:
    ///
    /// ```plain
    ///         ROOT       *ROOT*           ROOT          ROOT         
    ///      ┌───┼───┐     ┌─┼─┐         ┌───┼───┐     ┌───┼───┐       
    ///      A   B   C     A B C        *A*  B   C     A   B  *C*      
    ///   ┌──┼──┐                     ┌──┼──┐               ┌──┼──┐    
    ///  *D* E  F                     D  E  F               J  K  L    
    ///                                                                
    ///        (5)          (6)            (7)             (8)         
    /// ```
    ///
    /// - `(5)` = `(1)` + `jump(Direction::Up)` (wrap from `ROOT` to the
    ///   deepest leaf [`node`] in the first-child based `path`)
    /// - `(6)` = `(2)` + `jump(Direction::Down(...))` (return to the `ROOT`
    ///   because `G` has no children)
    /// - `(7)` = `(3)` + `jump(Direction::Right)` (wrap to the first [`node`]
    ///   at the same depth)
    /// - `(8)` = `(4)` + `jump(Direction::Left)` (wrap to the last [`node`]
    ///   at the same depth)
    ///
    /// [`cursor`]: Cursor
    /// [`direction`]: Direction
    /// [`walk()`]: Cursor::walk()
    /// [`node`]: Node
    /// [`nodes`]: Node
    /// [`tree`]: crate::Tree
    /// [`tree's`]: crate::Tree
    /// [`populating`]: crate::Populate
    pub fn jump(&mut self, direction: Direction) -> Result<bool, Error> {
        match direction {
            Direction::Up => self.jump_up(),
            Direction::Down(target) => self.jump_down(target),
            Direction::Left => self.jump_left(),
            Direction::Right => self.jump_right(),
        }
    }

    /// # Attempts to move the [`cursor`] to the child [`node`] selected by [`target`]
    ///
    /// The [`cursor`] remains unchanged and attempt to move is considered
    /// failed when the selected child does not exist. On success, the
    /// [`cursor`] registers a visit to the child and appends its index to the
    /// current `path`.
    ///
    /// Returns the success of attempt or an `error` if [`populating`] a child
    /// [`node`] fails.
    ///
    /// [`cursor`]: Cursor
    /// [`node`]: Node
    /// [`target`]: Target
    /// [`populating`]: crate::Populate
    fn down(&mut self, target: Target) -> Result<bool, Error> {
        let (children_len, child) = {
            let children = unsafe { &*self.children.get() };

            let children_len = children.len();
            let child = match target {
                Target::First => children.first().cloned(),
                Target::Last => children.last().cloned(),
                Target::Index(i) => children.get(i).cloned(),
            };

            (children_len, child)
        };

        if let Some(child) = child {
            unsafe { child.as_ref().visit(child)? };
            self.link = child;
            self.path.push(match target {
                Target::First => 0,
                Target::Last => children_len - 1,
                Target::Index(i) => i,
            });

            return Ok(true);
        }

        Ok(false)
    }

    /// # Moves the [`cursor`] to the child [`node`] selected by [`target`] or the [`tree's`] `root`
    ///
    /// Always returns `true` or an `error` if [`populating`] a child [`node`]
    /// fails.
    ///
    /// [`cursor`]: Cursor
    /// [`node`]: Node
    /// [`target`]: Target
    /// [`tree's`]: crate::Tree
    /// [`populating`]: crate::Populate
    fn jump_down(&mut self, target: Target) -> Result<bool, Error> {
        if !self.down(target)? {
            self.go_to(&[0])?;
        }

        Ok(true)
    }

    /// # Attempts to move the [`cursor`] to the parent [`node`]
    ///
    /// Returns `false` when the current [`node`] does not have a parent
    /// ([`tree's`] `root`); in that case the [`cursor`] remains unchanged.
    /// Otherwise returns `true`.
    ///
    /// [`cursor`]: Cursor
    /// [`node`]: Node
    /// [`tree's`]: crate::Tree
    fn up(&mut self) -> bool {
        unsafe {
            if let Some(parent) = self.link.as_ref().parent {
                self.link.as_ref().leave();
                self.link = parent;
                self.path.pop();

                return true;
            }
        }

        false
    }

    /// # Moves the [`cursor`] to the parent [`node`] or the deepest leaf [`node`]
    ///
    /// If the [`cursor`] is already at the [`tree's`] `root`, descends though
    /// first children until reaching the deepest leaf [`node`].
    ///
    /// Returns `true` or an `error` if [`populating`] a child [`node`] during
    /// descend fails.
    ///
    /// [`cursor`]: Cursor
    /// [`node`]: Node
    /// [`tree's`]: crate::Tree
    /// [`populating`]: crate::Populate
    fn jump_up(&mut self) -> Result<bool, Error> {
        if !self.up() {
            while self.down(Target::First)? {}
        }

        Ok(true)
    }

    /// # Attempts to move the [`cursor`] to the next sibling [`node`] in same branch
    ///
    /// The [`cursor`] remains unchanged and attempt to move is considered
    /// failed when the current [`node`] is the last child or is the [`tree's`]
    /// `root`.
    ///
    /// Returns the success of attempt or an `error` if [`populating`] a next
    /// sibling [`node`] fails.
    ///
    /// [`cursor`]: Cursor
    /// [`node`]: Node
    /// [`tree's`]: crate::Tree
    /// [`populating`]: crate::Populate
    fn right(&mut self) -> Result<bool, Error> {
        unsafe {
            if let Some(parent) = self.link.as_ref().parent {
                let i = self.path.last_mut().unwrap();

                let children = &*parent.as_ref().children.get();

                if let Some(new_node) = children.get(*i + 1) {
                    new_node.as_ref().visit(*new_node)?;
                    self.link.as_ref().leave();
                    self.link = *new_node;
                    *i += 1;

                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// # Moves the [`cursor`] to the next sibling [`node`] in same or adjacent branch
    ///
    /// When the current [`node`] is the last child, the [`cursor`] moves upward
    /// untill a branch with a next sibling is found, then descends through
    /// first children to preserve the original depth when possible.
    ///
    /// If no branch is available to the right, the traversal wraps around the
    /// [`tree`].
    ///
    /// Always returns `true` or an `error` if [`populating`] a new [`node`]
    /// fails.
    ///
    /// [`cursor`]: Cursor
    /// [`node`]: Node
    /// [`tree`]: crate::Tree
    /// [`populating`]: crate::Populate
    fn jump_right(&mut self) -> Result<bool, Error> {
        let initial_path_len = self.path.len();

        unsafe {
            while let Some(parent) = self.link.as_ref().parent {
                let i = self.path.last_mut().unwrap();

                let children = &*parent.as_ref().children.get();

                let new_node = children.get(*i + 1).cloned();

                if let Some(new_node) = new_node {
                    self.link.as_ref().leave();
                    self.link = new_node;
                    self.link.as_ref().visit(self.link)?;
                    *i += 1;
                    break;
                } else {
                    self.link.as_ref().leave();
                    self.link = parent;
                    self.path.pop();
                }
            }

            while self.path.len() != initial_path_len {
                let child = (&*self.children.get()).first().cloned();

                if let Some(child) = child {
                    self.link = child;
                    self.link.as_ref().visit(self.link)?;
                    self.path.push(0);
                } else {
                    break;
                }
            }
        }

        Ok(true)
    }

    /// # Attempts to move the [`cursor`] to the previous sibling [`node`] in same branch
    ///
    /// The [`cursor`] remains unchanged and attempt to move is considered
    /// failed when the currrent [`node`] is the first child or is the `root`.
    ///
    /// Returns the success of attempt or an `error` if [`populating`] a
    /// previous sibling [`node`] fails.
    ///
    /// [`cursor`]: Cursor
    /// [`node`]: Node
    /// [`populating`]: crate::Populate
    fn left(&mut self) -> Result<bool, Error> {
        unsafe {
            if let Some(parent) = self.link.as_ref().parent {
                let i = self.path.last_mut().unwrap();

                if *i > 0 {
                    let children = &*parent.as_ref().children.get();

                    if let Some(new_node) = children.get(*i - 1) {
                        self.link.as_ref().leave();
                        self.link = *new_node;
                        self.link.as_ref().visit(self.link)?;
                        *i -= 1;

                        return Ok(true);
                    }
                }
            }
        }

        Ok(false)
    }

    /// # Moves the [`cursor`] to the previous sibling [`node`] in same or adjacent branch
    ///
    /// When the current [`node`] is the first child, the [`cursor`] moves
    /// upward untill a branch with a previous sibling is found, then descends
    /// though last children to preserve the original depth when possible.
    ///
    /// If no branch is available to the left, the traversal wraps around the
    /// [`tree`].
    ///
    /// Always returns `true` or an `error` if [`populating`] a new [`node`]
    /// fails.
    ///
    /// [`cursor`]: Cursor
    /// [`node`]: Node
    /// [`tree`]: crate::Tree
    /// [`populating`]: crate::Populate
    fn jump_left(&mut self) -> Result<bool, Error> {
        let initial_path_len = self.path.len();

        unsafe {
            while let Some(parent) = self.link.as_ref().parent {
                let i = self.path.last_mut().unwrap();

                if *i > 0 {
                    let children = &*parent.as_ref().children.get();

                    let new_node = children.get(*i - 1).cloned();

                    if let Some(new_node) = new_node {
                        self.link.as_ref().leave();
                        self.link = new_node;
                        self.link.as_ref().visit(self.link)?;
                        *i -= 1;
                        break;
                    } else {
                        unreachable!()
                    }
                } else {
                    self.link.as_ref().leave();
                    self.link = parent;
                    self.path.pop();
                }
            }

            while self.path.len() != initial_path_len {
                let (child, i) = {
                    let children = &*self.children.get();
                    (children.last().cloned(), children.len())
                };

                if let Some(child) = child {
                    self.link = child;
                    self.link.as_ref().visit(self.link)?;
                    self.path.push(i - 1);
                } else {
                    break;
                }
            }
        }

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::cursor::vec_equ;

    #[test]
    fn test_vec_equ() {
        assert_eq!(vec_equ(&[0, 1, 2, 3, 4, 5], &[0, 1, 2]), None);
        assert_eq!(vec_equ(&[0, 1, 2, 3, 4, 5], &[0, 1, 3]), Some(2));
        assert_eq!(vec_equ(&[], &[0, 1, 3]), Some(0));
    }
}
