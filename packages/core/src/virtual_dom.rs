/*
The Dioxus Virtual Dom integrates an event system and virtual nodes to create reactive user interfaces.

The Dioxus VDom uses the same underlying mechanics as Dodrio (double buffering, bump dom, etc).
Instead of making the allocator very obvious, we choose to parametrize over the DomTree trait. For our purposes,
the DomTree trait is simply an abstraction over a lazy dom builder, much like the iterator trait.

This means we can accept DomTree anywhere as well as return it. All components therefore look like this:
```ignore
function Component(ctx: Context<()>) -> VNode {
    ctx.view(html! {<div> "hello world" </div>})
}
```
It's not quite as sexy as statics, but there's only so much you can do. The goal is to get statics working with the FC macro,
so types don't get in the way of you and your component writing. Fortunately, this is all generic enough to be split out
into its own lib (IE, lazy loading wasm chunks by function (exciting stuff!))

```ignore
#[fc] // gets translated into a function.
static Component: FC = |ctx| {
    ctx.view(html! {<div> "hello world" </div>})
}
```
*/
use crate::inner::*;
use crate::nodes::VNode;
use any::Any;
use bumpalo::Bump;
use generational_arena::{Arena, Index};
use std::{
    any::{self, TypeId},
    cell::{RefCell, UnsafeCell},
    future::Future,
    marker::PhantomData,
    sync::atomic::AtomicUsize,
};

/// An integrated virtual node system that progresses events and diffs UI trees.
/// Differences are converted into patches which a renderer can use to draw the UI.
pub struct VirtualDom<P: Properties> {
    /// All mounted components are arena allocated to make additions, removals, and references easy to work with
    /// A generational arean is used to re-use slots of deleted scopes without having to resize the underlying arena.
    components: Arena<Scope>,

    /// The index of the root component.
    base_scope: Index,

    /// Components generate lifecycle events
    event_queue: Vec<LifecycleEvent>,

    root_props: P,
}

/// Implement VirtualDom with no props for components that initialize their state internal to the VDom rather than externally.
impl VirtualDom<()> {
    /// Create a new instance of the Dioxus Virtual Dom with no properties for the root component.
    ///
    /// This means that the root component must either consumes its own context, or statics are used to generate the page.
    /// The root component can access things like routing in its context.
    pub fn new(root: FC<()>) -> Self {
        Self::new_with_props(root, ())
    }
}

/// Implement the VirtualDom for any Properties
impl<P: Properties + 'static> VirtualDom<P> {
    /// Start a new VirtualDom instance with a dependent props.
    /// Later, the props can be updated by calling "update" with a new set of props, causing a set of re-renders.
    ///
    /// This is useful when a component tree can be driven by external state (IE SSR) but it would be too expensive
    /// to toss out the entire tree.
    pub fn new_with_props(root: FC<P>, root_props: P) -> Self {
        // 1. Create the component arena
        // 2. Create the base scope (can never be removed)
        // 3. Create the lifecycle queue
        // 4. Create the event queue

        // Arena allocate all the components
        // This should make it *really* easy to store references in events and such
        let mut components = Arena::new();

        // Create a reference to the component in the arena
        let base_scope = components.insert(Scope::new(root, None));

        // Create a new mount event with no root container
        let first_event = LifecycleEvent::mount(base_scope, None, 0);

        // Create an event queue with a mount for the base scope
        let event_queue = vec![first_event];

        Self {
            components,
            base_scope,
            event_queue,
            root_props,
        }
    }

    /// Pop an event off the even queue and process it
    pub fn progress(&mut self) -> Result<(), ()> {
        let LifecycleEvent { index, event_type } = self.event_queue.pop().ok_or(())?;

        let scope = self.components.get(index).ok_or(())?;

        match event_type {
            // Component needs to be mounted to the virtual dom
            LifecycleType::Mount { to, under } => {
                // todo! run the FC with the bump allocator
                // Run it with its properties
                if let Some(other) = to {
                    // mount to another component
                    let p = ();
                } else {
                    // mount to the root
                }
            }

            // The parent for this component generated new props and the component needs update
            LifecycleType::PropsChanged {} => {}

            // Component was successfully mounted to the dom
            LifecycleType::Mounted {} => {}

            // Component was removed from the DOM
            // Run any destructors and cleanup for the hooks and the dump the component
            LifecycleType::Removed {} => {
                let f = self.components.remove(index);
            }

            // Component was messaged via the internal subscription service
            LifecycleType::Messaged => {}
        }

        Ok(())
    }

    /// Update the root props, causing a full event cycle
    pub fn update_props(&mut self, new_props: P) {}

    /// Run through every event in the event queue until the events are empty.
    /// Function is asynchronous to allow for async components to finish their work.
    pub async fn progess_completely() {}

    /// Create a new context object for a given component and scope
    fn new_context<T: Properties>(&self) -> Context<T> {
        todo!()
    }

    /// Stop writing to the current buffer and start writing to the new one.
    /// This should be done inbetween CallbackEvent handling, but not between lifecycle events.
    pub fn swap_buffers(&mut self) {}
}

pub struct LifecycleEvent {
    pub index: Index,
    pub event_type: LifecycleType,
}
impl LifecycleEvent {
    fn mount(which: Index, to: Option<Index>, under: usize) -> Self {
        Self {
            index: which,
            event_type: LifecycleType::Mount { to, under },
        }
    }
}
/// The internal lifecycle event system is managed by these
pub enum LifecycleType {
    Mount { to: Option<Index>, under: usize },
    PropsChanged,
    Mounted,
    Removed,
    Messaged,
}
