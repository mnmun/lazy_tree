//! # [`Tree`] traversing tool
//!
//! ![](https://github.com/mnmun/images/blob/main/index_left.png?raw=true)
//!
//! Defines a [`cursor`] - the tool for traversing a [`tree`] of lazily-created
//! [`nodes`]. It can move between [`nodes`] within the [`tree`] and provides
//! access to the `value` stored in the [`node`] it currently points to. As it
//! moves, the [`cursor`] triggers the [creation] of any [`nodes`] it need
//! to access, as well as the deletion of [`nodes`] that are no longer in use.
//!
//! ---
//!
//! See the [`crate`] documentation for more information.
//!
//! [`tree`]: crate::Tree
//! [`cursor`]: crate::Cursor
//! [`node`]: crate::Node
//! [`nodes`]: crate::Node
//! [creation]: crate::callbacks::Query

use std::{marker::PhantomData, ops::Deref};

use getset::Getters;

use crate::node::Link;

#[cfg(test)]
use crate::Node;

/// # Specifies which child [`cursor`] selects when moving down
///
/// ![](https://github.com/mnmun/images/blob/main/target.png?raw=true)
///
/// See the [`direction`] documentation for more information.
///
/// [`node`]: crate::Node
/// [`cursor`]: crate::Cursor
/// [`direction`]: crate::cursor::Direction
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
/// ![](https://github.com/mnmun/images/blob/main/no_crossing.png?raw=true)
///
/// See the [`walk()`] and [`jump()`] methods documentation for more
/// information.
///
/// [`cursor`]: crate::Cursor
/// [`walk()`]: crate::Cursor::walk()
/// [`jump()`]: crate::Cursor::jump()
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Direction {
    /// Moves the [`cursor`] to the parent [`node`]
    ///
    /// [`cursor`]: crate::Cursor
    /// [`node`]: crate::Node
    Up,

    /// Moves the [`cursor`] to its child [`node`] selected by [`target`]
    ///
    /// [`cursor`]: crate::Cursor
    /// [`node`]: crate::Node
    /// [`target`]: crate::cursor::Target
    Down(Target),

    /// Moves the [`cursor`] to the previous sibling [`node`]
    ///
    /// [`cursor`]: crate::Cursor
    /// [`node`]: crate::Node
    Left,

    /// Moves the [`cursor`] to the next sibling [`node`]
    ///
    /// [`cursor`]: crate::Cursor
    /// [`node`]: crate::Node
    Right,
}

/// # `Cursor`
///
/// ![](https://github.com/mnmun/images/blob/main/index_left.png?raw=true)
///
/// Traverses a [`tree`] of lazily-created [`nodes`]. It can move between
/// [`nodes`] within the [`tree`] and provides access to the `value` stored in
/// the [`node`] it currently points to. As it moves, the `cursor` triggers
/// the [creation] of any [`nodes`] it need to access, as well as the deletion
/// of [`nodes`] that are no longer in use.
///
/// `Cursor` holds:
///
/// - [`link`] to the [`node`] it currently points;
/// - [`path`] from the [`tree's`] `root` [`node`] to the [`node`] it currently
///   points to.
///
/// `Cursor` dereferences to a `&Value`, allowing direct access to the
/// [`node's`] `value`.
///
/// [`Path`] contains one zero-based child index for each passed level in the
/// [`tree`]. Valid [`path`] could not be empty and should start with zero.
///
/// ## Creation
///
/// Use the [`Tree::cursor()`] method to create a new `cursor` pointing to the
/// [`tree's`] `root` [`node`] with the [`path`] set to `[0]`.
///
/// [`Tree`] can have multiple `cursors` at the same time. `Cursors` implement
/// the [`Send`] trait, so they can be sent between threads safely. Each
/// `cursor` remains borrowed from its [`tree`] and cannot outlive it.
///
/// ## Traversing
///
/// `Cursor` has several methods for moving between [`nodes`] in the [`tree`]:
///
/// - [`go_to()`] - moves `cursor` to a [`node`] by a provided absolute `path`;
/// - [`walk()`] - attempts to perform a movement within the current branch in
///   the given `direction`;
/// - [`jump()`] - performs a movement in the same way as the `walk()` method;
///   in cases where such movement is not possible jumps to an adjacent branch.
///
/// ---
///
/// See the [`module`] documentation for more information.
///
/// [`module`]: crate::cursor
/// [`link`]: crate::node::Link
/// [`node`]: crate::Node
/// [`node's`]: crate::Node
/// [`nodes`]: crate::Node
/// [`&Node`]: crate::Node
/// [`path`]: crate::Cursor::path()
/// [`paths`]: crate::Cursor::path()
/// [`Path`]: crate::Cursor::path()
/// [`tree`]: crate::Tree
/// [`tree's`]: crate::Tree
/// [`Tree`]: crate::Tree
/// [`Tree::cursor()`]: crate::Tree::cursor()
/// [`go_to()`]: crate::Cursor::go_to()
/// [`walk()`]: crate::Cursor::walk()
/// [`jump()`]: crate::Cursor::jump()
/// [`direction`]: crate::cursor::Direction
/// [creation]: crate::callbacks::Query
#[derive(Debug, Getters)]
pub struct Cursor<'tree, Value, Error> {
    /// # [`Link`] to the [`node`] the [`cursor`] is currently pointing to
    ///
    /// See the [`cursor`] documentation for more information.
    ///
    /// [`Link`]: crate::node::Link
    /// [`node`]: crate::Node
    /// [`cursor`]: crate::Cursor
    pub(crate) link: Link<Value, Error>,

    /// # Absolute `path` from the [`tree's`] `root` [`node`] to the [`node`] it currently points to
    ///
    /// Every subsequent element is the zero-based `index` of the selected child
    /// [`node`] at that [`tree`] level.
    ///
    /// The `root` [`node`] is located at the path `[0]`.
    ///
    /// ---
    ///
    /// See the [`cursor`] documentation for more information.
    ///
    /// [`cursor`]: crate::Cursor
    /// [`tree`]: crate::Tree
    /// [`tree's`]: crate::Tree
    /// [`node`]: crate::Node
    #[getset(get = "pub")]
    path: Vec<usize>,

    /// # Important!
    ///
    /// Prevents the [`cursor`] from outliving the [`tree`] from which it was
    /// created.
    ///
    /// ---
    ///
    /// See the [`cursor`] documentation for more information.
    ///
    /// [`cursor`]: crate::Cursor
    /// [`tree`]: crate::Tree
    _marker: PhantomData<&'tree ()>,
}

/// # Safety
///
/// [`Cursor`] contains a raw [`non-null`] pointer to a [`node`] (in the form
/// of a [`link`]); it is safe to pass between threads because the [`node`] the
/// [`cursor`] points to cannot be deleted before the [`cursor`] itself, and the
/// [`cursor`] cannot outlive the [`tree`] from which it was created.
///
/// ---
///
/// See the [`cursor`] documentation for more information.
///
/// [`cursor`]: crate::Cursor
/// [`non-null`]: std::ptr::NonNull
/// [`node`]: crate::Node
/// [`link`]: crate::node::Link
/// [`tree`]: crate::Tree
unsafe impl<'tree, Value, Error> Send for Cursor<'tree, Value, Error> {}

impl<'tree, Value, Error> Drop for Cursor<'tree, Value, Error> {
    /// # Leaves all its [`nodes`]
    ///
    /// The [`cursor`] returns to the `root` [`node`] and then leaves it.
    ///
    /// ---
    ///
    /// See the [`cursor`] documentation for more information.
    ///
    /// [`node`]: crate::Node
    /// [`nodes`]: crate::Node
    /// [`cursor`]: crate::Cursor
    fn drop(&mut self) {
        let _ = self.go_to(&[0]);
        unsafe { self.link.as_mut().leave() }
    }
}

impl<'tree, Value, Error> Deref for Cursor<'tree, Value, Error> {
    type Target = Value;

    fn deref(&self) -> &Self::Target {
        unsafe { &self.link.as_ref().value }
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

impl<'tree, Value, Error> Cursor<'tree, Value, Error> {
    /// # Creates a [`cursor`] pointed to the [`link`] with the given `path`
    ///
    /// Calls the [`enter()`] method for the [`node`], a [`link`] to which
    /// was passed.
    ///
    /// ## Errors
    ///
    /// Returns the [`total`] callback error unchanged. In that case, no
    /// [`cursor`] is produced.
    ///
    /// ---
    ///
    /// See the [`cursor`] documentation for more information.
    ///
    /// [`cursor`]: crate::Cursor
    /// [`link`]: crate::node::Link
    /// [`enter()`]: crate::Node::enter()
    /// [`node`]: crate::Node
    /// [`total`]: crate::callbacks::Total
    pub(crate) fn new(
        mut link: Link<Value, Error>,
        path: Vec<usize>,
    ) -> Result<Self, Error> {
        unsafe { link.as_mut().enter()? };

        Ok(Cursor {
            link,
            path,
            _marker: PhantomData,
        })
    }

    /// # Exposes the [`total`] value of the [`node`] being pointed to
    ///
    /// Indicates the [`total`] number of possible child node.
    ///
    /// ---
    ///
    /// See the [`cursor`] documentation for more information.
    ///
    /// [`cursor`]: crate::Cursor
    /// [`total`]: crate::callbacks::Total
    /// [`node`]: crate::Node
    pub fn total(&self) -> Option<usize> {
        unsafe { self.link.as_ref().total }
    }

    /// # Attempts to move the [`cursor`] by an absolute `path`
    ///
    /// [`Cursor`] follows the provided absolute `path`; if a [`node`] cannot be
    /// [`queried`] at any step, the attempt is considered **failed** and the
    /// [`cursor`] remains at the last successfully reached [`node`].
    ///
    /// If the provided absolute `path` does not start from zero, the [`cursor`]
    /// is returned to the `root` [`node`] and the attempt is considered
    /// **failed**.
    ///
    /// Returns the success of the attempt or an `error` from [`Query`] or
    /// [`Total`] callbacks if any of them fail.
    ///
    /// ---
    ///
    /// See the [`cursor`] documentation for more information.
    ///
    /// [`cursor`]: crate::Cursor
    /// [`node`]: crate::Node
    /// [`queried`]: crate::callbacks::Query
    /// [`Query`]: crate::callbacks::Query
    /// [`Total`]: crate::callbacks::Total
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

    /// # Attempts to move the [`cursor`] within the current branch in the given [`direction`]
    ///
    /// The movement of the [`cursor`], depending on the given [`direction`] can
    /// be described by the following table:
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
    /// If the [`queried`] [`node`] does not exist, the [`cursor`] remains at
    /// its current position and the attempt to move is considered **failed**.
    /// Otherwise the [`cursor`] successfully moves to the target [`node`].
    ///
    /// Returns the success of the attempt or an `error` from [`Query`] or
    /// [`Total`] callbacks if any of them fail.
    ///
    /// ## Details
    ///
    /// For the sake of illustration, let us consider a sample [`tree`]
    /// containing a single [`cursor`], which is currently pointing to node `B`.
    /// We will denote all [`nodes`] currently existing in memory using
    /// parentheses `( )`; for the [`node`] currently pointed to by the
    /// [`cursor`], we will add asterisks `* *` alongside the parentheses:
    ///
    /// ```plain
    ///                 (ROOT)             
    ///       ┌───────────┼───────────┐     
    ///       A         *(B)*         C     
    ///   ┌───┼───┐   ┌───┼───┐   ┌───┼───┐
    ///   D   E   F   G   H   I   J   K   L
    ///
    ///                   0)
    /// ```
    ///
    /// The diagram above shows a [`tree`] in which only two [`nodes`] currently
    /// exist: `ROOT` and `B` because they are used by the [`cursor`]. The other
    /// [`nodes`] are shown only to demonstrate the complete shape of the
    /// [`tree`].
    ///
    /// The following [`cursor`] movements are successful, as they occur within
    /// the bounds of the [`cursor's`] current branch:
    ///
    /// - `1)` = `0)` + `walk(Direction::Up)`
    /// - `2)` = `0)` + `walk(Direction::Down(Target::First))`
    /// - `3)` = `0)` + `walk(Direction::Right)`
    /// - `4)` = `0)` + `walk(Direction::Left)`
    ///
    /// where
    ///
    /// ```plain
    ///                *(ROOT)*                               (ROOT)         
    ///       ┌───────────┼───────────┐             ┌───────────┼───────────┐    
    ///       A           B           C             A          (B)          C    
    ///   ┌───┼───┐   ┌───┼───┐   ┌───┼───┐     ┌───┼───┐   ┌───┼───┐   ┌───┼───┐
    ///   D   E   F   G   H   I   J   K   L     D   E   F *(G)* H   I   J   K   L
    ///                                                           
    ///                   1)                                    2)            
    ///
    ///
    ///                 (ROOT)                                (ROOT)            
    ///       ┌───────────┼───────────┐             ┌───────────┼───────────┐    
    ///       A           B         *(C)*         *(A)*         B           C    
    ///   ┌───┼───┐   ┌───┼───┐   ┌───┼───┐     ┌───┼───┐   ┌───┼───┐   ┌───┼───┐
    ///   D   E   F   G   H   I   J   K   L     D   E   F   G   H   I   J   K   L
    ///                                                           
    ///                   3)                                    4)            
    /// ```
    ///
    /// Keep in mind that horizontal movements are internally implemented as a
    /// sequence of vertical movements. They can therefore be expressed as
    /// follows:
    ///
    /// - `3)` = `0)` + `walk(Direction::Up)` + `walk(Direction::Down(Target::Index(i + 1)))`
    /// - `4)` = `0)` + `walk(Direction::Up)` + `walk(Direction::Down(Target::Index(i - 1)))`
    ///
    /// where `i` is `index` of the node `B`, in this case `i = 1`.
    ///
    /// In the following cases, the queried [`node`] in the current branch does
    /// not exist. The [`cursor`] therefore remains in place and the `walk()`
    /// method returns `false`:
    ///
    /// - `1)` + `walk(Direction::Up)`
    /// - `2)` + `walk(Direction::Down(...))`
    /// - `2)` + `walk(Direction::Left)`
    /// - `3)` + `walk(Direction::Right)`
    /// - `4)` + `walk(Direction::Left)`
    ///
    /// ---
    ///
    /// See the [`cursor`] documentation for more information.
    ///
    /// [`cursor`]: crate::Cursor
    /// [`cursor's`]: crate::Cursor
    /// [`node`]: crate::Node
    /// [`nodes`]: crate::Node
    /// [`direction`]: crate::cursor::Direction
    /// [`target`]: crate::cursor::Target
    /// [`tree`]: crate::Tree
    /// [`Query`]: crate::callbacks::Query
    /// [`queried`]: crate::callbacks::Query
    /// [`Total`]: crate::callbacks::Total
    pub fn walk(&mut self, direction: Direction) -> Result<bool, Error> {
        match direction {
            Direction::Up => Ok(self.up()),
            Direction::Down(target) => self.down(target),
            Direction::Left => self.left(),
            Direction::Right => self.right(),
        }
    }

    /// # Moves the [`cursor`] in the given [`direction`] with a possible branch switch
    ///
    /// Behaves in the same way as the [`walk()`] method for those cases where
    /// [`walk()`] returns `true`.
    ///
    /// For all other cases (where the [`walk()`] method cannot perform the
    /// requested movement) the [`cursor's`] movement, depending on the given
    /// [`direction`] can be described by the following table:
    ///
    /// ```plain
    /// ┌───────────┬─────────────────────────────────────────────────────────┐
    /// │ Direction │ Moves cursor to the                                     │
    /// ├───────────┼─────────────────────────────────────────────────────────┤
    /// │ Up        │ Deepest leaf node in the first-child based path         │
    /// ├───────────┼─────────────────────────────────────────────────────────┤
    /// │ Down      │ Tree's root                                             │
    /// ├───────────┼─────────────────────────────────────────────────────────┤
    /// │ Right     │ Node of the next adjacent branch got by descending      │
    /// │           │ through first children                                  │
    /// ├───────────┼─────────────────────────────────────────────────────────┤
    /// │ Left      │ Node of the previous adjacent branch got by descending  │
    /// │           │ through last children                                   │
    /// └───────────┴─────────────────────────────────────────────────────────┘
    /// ```
    ///
    /// When no branch exists in the requested horizontal [`direction`], the
    /// search wraps around the [`tree`].
    ///
    /// Returns `true` or an `error` from [`Query`] or [`Total`] callbacks if
    /// any of them fail.
    ///
    /// ## Details
    ///
    /// The four diagrams below show the results of successful calls to the
    /// [`walk()`] method, as described in the [`walk()`] method documentation:
    ///
    /// ```plain
    ///                *(ROOT)*                               (ROOT)         
    ///       ┌───────────┼───────────┐             ┌───────────┼───────────┐    
    ///       A           B           C             A          (B)          C    
    ///   ┌───┼───┐   ┌───┼───┐   ┌───┼───┐     ┌───┼───┐   ┌───┼───┐   ┌───┼───┐
    ///   D   E   F   G   H   I   J   K   L     D   E   F *(G)* H   I   J   K   L
    ///                                                           
    ///                   1)                                    2)            
    ///
    ///
    ///                 (ROOT)                                (ROOT)         
    ///       ┌───────────┼───────────┐             ┌───────────┼───────────┐    
    ///       A           B         *(C)*         *(A)*         B           C    
    ///   ┌───┼───┐   ┌───┼───┐   ┌───┼───┐     ┌───┼───┐   ┌───┼───┐   ┌───┼───┐
    ///   D   E   F   G   H   I   J   K   L     D   E   F   G   H   I   J   K   L
    ///                                                           
    ///                   3)                                    4)            
    /// ```
    ///
    /// *Reminder*: In the diagram above, all [`nodes`] currently existing in
    /// memory are denoted with parentheses `( )`, while the [`node`] pointed
    /// to by the [`cursor`] is denoted with both parentheses and asterisks
    /// `*( )*`. [`Nodes`] without such markings do not exist in memory; they
    /// are shown solely to illustrate the complete shape of the [`tree`].
    ///
    /// Applying the `jump()` method, to the cases where the [`walk()`] method
    /// could not move, produces the following states:
    ///
    /// - `5)` = `1)` + `jump(Direction::Up)`
    /// - `6)` = `2)` + `jump(Direction::Down(...))`
    /// - `7)` = `2)` + `jump(Direction::Left)`
    /// - `8)` = `3)` + `jump(Direction::Right)`
    /// - `9)` = `4)` + `jump(Direction::Left)`
    ///
    /// where
    ///
    /// ```plain
    ///                 (ROOT)                               *(ROOT)*            
    ///       ┌───────────┼───────────┐             ┌───────────┼───────────┐    
    ///      (A)          B           C             A           B           C    
    ///   ┌───┼───┐   ┌───┼───┐   ┌───┼───┐     ┌───┼───┐   ┌───┼───┐   ┌───┼───┐
    /// *(D)* E   F   G   H   I   J   K   L     D   E   F   G   H   I   J   K   L
    ///                                                                          
    ///                   5)                                    6)               
    ///
    ///
    ///                 (ROOT)                                (ROOT)         
    ///       ┌───────────┼───────────┐             ┌───────────┼───────────┐    
    ///      (A)          B           C           *(A)*         B           C    
    ///   ┌───┼───┐   ┌───┼───┐   ┌───┼───┐     ┌───┼───┐   ┌───┼───┐   ┌───┼───┐
    ///   D   E *(F)* G   H   I   J   K   L     D   E   F   G   H   I   J   K   L
    ///                                                           
    ///                   7)                                    8)            
    ///
    ///
    ///                                    (ROOT)              
    ///                          ┌───────────┼───────────┐     
    ///                          A           B         *(C)*   
    ///                      ┌───┼───┐   ┌───┼───┐   ┌───┼───┐
    ///                      D   E   F   G   H   I   J   K   L
    ///                                                        
    ///                                      9)                
    /// ```
    ///
    /// ---
    ///
    /// See the [`cursor`] documentation for more information.
    ///
    /// [`cursor`]: crate::Cursor
    /// [`cursor's`]: crate::Cursor
    /// [`direction`]: crate::cursor::Direction
    /// [`walk()`]: crate::Cursor::walk()
    /// [`node`]: crate::Node
    /// [`nodes`]: crate::Node
    /// [`tree`]: crate::Tree
    /// [`tree's`]: crate::Tree
    /// [`Query`]: crate::callbacks::Query
    /// [`Total`]: crate::callbacks::Total
    pub fn jump(&mut self, direction: Direction) -> Result<bool, Error> {
        match direction {
            Direction::Up => self.jump_up(),
            Direction::Down(target) => self.jump_down(target),
            Direction::Left => self.jump_left(),
            Direction::Right => self.jump_right(),
        }
    }

    /// # Attempts to move the [`cursor`] to its child [`node`] selected by [`target`]
    ///
    /// The [`cursor`] moves to the [`target`] child [`node`], [`entering`] that
    /// [`node`]. If the `index` of the [`target`] child [`node`] is out of
    /// bounds (`0 <= index < total`), the attempt is considered **failed**, and
    /// the [`cursor`] remains unchanged.
    ///
    /// Returns the success of the attempt or an `error` from [`Query`] or
    /// [`Total`] callbacks if any of them fail.
    ///
    /// ---
    ///
    /// See the [`cursor`] documentation for more information.
    ///
    /// [`cursor`]: crate::Cursor
    /// [`node`]: crate::Node
    /// [`entering`]: crate::Node::enter()
    /// [`target`]: crate::cursor::Target
    /// [`Query`]: crate::callbacks::Query
    /// [`Total`]: crate::callbacks::Total
    fn down(&mut self, target: Target) -> Result<bool, Error> {
        unsafe {
            let total = self.link.as_ref().total.as_ref().unwrap();

            let index = match target {
                Target::First => 0,
                Target::Last => total.saturating_sub(1),
                Target::Index(i) => i,
            };

            if index >= *total {
                return Ok(false);
            }

            let mut children = match self.link.as_ref().children.lock() {
                Ok(guard) => guard,
                Err(poison) => poison.into_inner(),
            };

            let mut node = if let Some(node) = children.get(&index) {
                *node
            } else {
                let node = (self.link.as_ref().callbacks.query)(
                    &self.link.as_mut().value,
                    index,
                )?
                .with_parent(self.link)
                .build();

                children.insert(index, node);
                *children.get(&index).unwrap()
            };

            node.as_mut().enter()?;
            self.link = node;
            self.path.push(index);

            Ok(true)
        }
    }

    /// # Moves the [`cursor`] to the [`target`] child [`node`] or the [`tree`] `root`
    ///
    /// Returns `true` or an `error` from [`Query`] or [`Total`] callbacks if
    /// any of them fail.
    ///
    /// ---
    ///
    /// See the [`cursor`] documentation for more information.
    ///
    /// [`cursor`]: crate::Cursor
    /// [`node`]: crate::Node
    /// [`target`]: crate::cursor::Target
    /// [`tree`]: crate::Tree
    /// [`Query`]: crate::callbacks::Query
    /// [`Total`]: crate::callbacks::Total
    fn jump_down(&mut self, target: Target) -> Result<bool, Error> {
        if !self.down(target)? {
            self.go_to(&[0])?;
        }

        Ok(true)
    }

    /// # Attempts to move the [`cursor`] to its parent [`node`]
    ///
    /// The [`cursor`] moves to the parent [`node`], [`leaving`] the current
    /// child [`node`]. If there is no parent [`node`], the attempt is
    /// considered **failed**, and the [`cursor`] remains unchanged.
    ///
    /// Returns the success of the attempt or an `error` from [`Query`] or
    /// [`Total`] callbacks if any of them fail.
    ///
    /// ---
    ///
    /// See the [`cursor`] documentation for more information.
    ///
    /// [`cursor`]: crate::Cursor
    /// [`node`]: crate::Node
    /// [`leaving`]: crate::Node::leave()
    /// [`Query`]: crate::callbacks::Query
    /// [`Total`]: crate::callbacks::Total
    fn up(&mut self) -> bool {
        unsafe {
            if let Some(parent) = self.link.as_ref().parent {
                self.link.as_mut().leave();
                self.link = parent;
                self.link.as_mut().cleanup(self.path.pop().unwrap());

                return true;
            }
        }

        false
    }

    /// # Moves the [`cursor`] to its parent [`node`] or the deepest leaf [`node`]
    ///
    /// If the [`cursor`] is already at the [`tree`] `root` [`node`], descends
    /// though first child [`nodes`] until reaching the deepest leaf [`node`].
    /// Otherwise moves the [`cursor`] to its parent [`node`].
    ///
    /// Returns `true` or an `error` from [`Query`] or [`Total`] callbacks if
    /// any of them fail.
    ///
    /// ---
    ///
    /// See the [`cursor`] documentation for more information.
    ///
    /// [`cursor`]: crate::Cursor
    /// [`node`]: crate::Node
    /// [`nodes`]: crate::Node
    /// [`tree`]: crate::Tree
    /// [`Query`]: crate::callbacks::Query
    /// [`Total`]: crate::callbacks::Total
    fn jump_up(&mut self) -> Result<bool, Error> {
        if !self.up() {
            while self.down(Target::First)? {}
        }

        Ok(true)
    }

    /// # Attempts to move the [`cursor`] to the next sibling [`node`] in same branch
    ///
    /// Executes the [`up()`] method followed by [`down(Target::Index(i + 1))`],
    /// where `i` - is the `index` of the current [`node`] pointed to by the
    /// [`cursor`]. If the [`down(Target::Index(i + 1))`] movement cannot be
    /// performed, the [`cursor`] is returned to the previous [`node`] using
    /// [`down(Target::Index(i))`], and the attempt is considered **failed**.
    ///
    /// Returns the success of the attempt or an `error` from [`Query`] or
    /// [`Total`] callbacks if any of them fail.
    ///
    /// ---
    ///
    /// See the [`cursor`] documentation for more information.
    ///
    /// [`cursor`]: crate::Cursor
    /// [`node`]: crate::Node
    /// [`up()`]: crate::Cursor::up()
    /// [`down(Target::Index(i + 1))`]: crate::Cursor::down()
    /// [`down(Target::Index(i))`]: crate::Cursor::down()
    /// [`Query`]: crate::callbacks::Query
    /// [`Total`]: crate::callbacks::Total
    fn right(&mut self) -> Result<bool, Error> {
        let i = *self.path.last().unwrap();

        if !self.up() {
            return Ok(false);
        }

        if !self.down(Target::Index(i + 1))? {
            self.down(Target::Index(i))?;
            return Ok(false);
        }

        Ok(true)
    }

    /// # Moves the [`cursor`] to the next sibling [`node`] in the same or an adjacent branch
    ///
    /// Executes the [`up()`] method followed by [`down(Target::Index(i + 1))`],
    /// where `i` - is the `index` of the current [`node`] pointed to by the
    /// [`cursor`]. If the [`down(Target::Index(i + 1))`] movement cannot be
    /// performed, the [`cursor`] executes [`up()`] and tries again for a new
    /// `i` value in a loop until [`down(Target::Index(i + 1))`] is executed
    /// successfully or the [`cursor`] reaches the `root` [`node`]. After
    /// exiting the loop, the [`cursor`] descends to the same depth as it was
    /// at the beginning using [`down(Target::First)`] in the loop.
    ///
    /// Returns `true` or an `error` from [`Query`] or [`Total`] callbacks if
    /// any of them fail.
    ///
    /// ---
    ///
    /// See the [`cursor`] documentation for more information.
    ///
    /// [`cursor`]: crate::Cursor
    /// [`node`]: crate::Node
    /// [`up()`]: crate::Cursor::up()
    /// [`down(Target::Index(i + 1))`]: crate::Cursor::down()
    /// [`down(Target::Index(i))`]: crate::Cursor::down()
    /// [`down(Target::First)`]: crate::Cursor::down()
    /// [`Query`]: crate::callbacks::Query
    /// [`Total`]: crate::callbacks::Total
    fn jump_right(&mut self) -> Result<bool, Error> {
        let initial_path_len = self.path.len();

        let mut i = *self.path.last().unwrap();

        while self.up() {
            if self.down(Target::Index(i + 1))? {
                break;
            }

            i = *self.path.last().unwrap();
        }

        while self.path.len() != initial_path_len {
            if !self.down(Target::First)? {
                break;
            }
        }

        Ok(true)
    }

    /// # Attempts to move the [`cursor`] to the previous sibling [`node`] in same branch
    ///
    /// Executes the [`up()`] method followed by [`down(Target::Index(i - 1))`],
    /// where `i` - is the `index` of the current [`node`] pointed to by the
    /// [`cursor`]. If the [`down(Target::Index(i - 1))`] movement cannot be
    /// performed, the [`cursor`] is returned to the previous [`node`] using
    /// [`down(Target::Index(i))`], and the attempt is considered **failed**.
    ///
    /// Returns the success of the attempt or an `error` from [`Query`] or
    /// [`Total`] callbacks if any of them fail.
    ///
    /// ---
    ///
    /// See the [`cursor`] documentation for more information.
    ///
    /// [`cursor`]: crate::Cursor
    /// [`node`]: crate::Node
    /// [`up()`]: crate::Cursor::up()
    /// [`down(Target::Index(i - 1))`]: crate::Cursor::down()
    /// [`down(Target::Index(i))`]: crate::Cursor::down()
    /// [`Query`]: crate::callbacks::Query
    /// [`Total`]: crate::callbacks::Total
    fn left(&mut self) -> Result<bool, Error> {
        let i = *self.path.last().unwrap();

        if i == 0 {
            return Ok(false);
        }

        if !self.up() {
            return Ok(false);
        }

        if !self.down(Target::Index(i - 1))? {
            self.down(Target::Index(i))?;
            return Ok(false);
        }

        Ok(true)
    }

    /// # Moves the [`cursor`] to the previous sibling [`node`] in the same or an adjacent branch
    ///
    /// Executes the [`up()`] method followed by [`down(Target::Index(i - 1))`],
    /// where `i` - is the `index` of the current [`node`] pointed to by the
    /// [`cursor`]. If the [`down(Target::Index(i - 1))`] movement cannot be
    /// performed, the [`cursor`] executes [`up()`] and tries again for a new
    /// `i` value in a loop until [`down(Target::Index(i - 1))`] is executed
    /// successfully or the [`cursor`] reaches the `root` [`node`]. After
    /// exiting the loop, the [`cursor`] descends to the same depth as it was
    /// at the beginning using [`down(Target::Last)`] in the loop.
    ///
    /// Returns `true` or an `error` from [`Query`] or [`Total`] callbacks if
    /// any of them fail.
    ///
    /// ---
    ///
    /// See the [`cursor`] documentation for more information.
    ///
    /// [`cursor`]: crate::Cursor
    /// [`node`]: crate::Node
    /// [`up()`]: crate::Cursor::up()
    /// [`down(Target::Index(i - 1))`]: crate::Cursor::down()
    /// [`down(Target::Index(i))`]: crate::Cursor::down()
    /// [`down(Target::Last)`]: crate::Cursor::down()
    /// [`Query`]: crate::callbacks::Query
    /// [`Total`]: crate::callbacks::Total
    fn jump_left(&mut self) -> Result<bool, Error> {
        let initial_path_len = self.path.len();

        let mut i = *self.path.last().unwrap();

        while self.up() {
            if i != 0 {
                self.down(Target::Index(i - 1))?;
                break;
            }

            i = *self.path.last().unwrap();
        }

        while self.path.len() != initial_path_len {
            if !self.down(Target::Last)? {
                break;
            }
        }

        Ok(true)
    }
}

#[cfg(test)]
impl<Value, Error> Cursor<'_, Value, Error> {
    pub fn node(&self) -> &Node<Value, Error> {
        unsafe { self.link.as_ref() }
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
