//! The Events which can be created by the compiler for the purpose
//! of tracing and visibility into what the compiler is doing.

use std::{cell::RefCell, fmt::Debug, rc::Rc};

use crate::compiler::{CompilerDisplay, CompilerError, Span};

use super::{Writable, Writer};

/// The Event ID module.  This manages the creation of new EventIds and
/// making sure that event one is provided a value that is unique within
/// a single execution of the compiler.
///
/// The EventId is given its own module so that nothing other than the
/// EventId system can view or interact with the values or assignments
/// of the IDs themselves.
pub mod event_id {
    use std::sync::atomic::AtomicU64;

    use super::{Writable, Writer};

    /// Threadsafe mechanism for providing unique IDs for every event
    static NEXT_EVENT_ID: AtomicU64 = AtomicU64::new(1);

    /// Uniquely identifies each [`Event`] generated by the compiler.
    /// This unique ID can then be used for exact causative connections
    /// between events.
    #[derive(PartialEq, Clone, Copy, Debug)]
    pub struct EventId(u64);

    impl EventId {
        pub(super) fn new() -> EventId {
            // Get a new unique event id
            let id = NEXT_EVENT_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            EventId(id)
        }
    }

    impl Writable for EventId {
        fn write(&self, w: &dyn Writer) {
            w.write_u64(self.0);
        }
    }
}

/// An event from any stage in the Compiler caused by the given span of source
/// code.
pub struct Event<'a, V: Writable, E: CompilerDisplay + Debug> {
    /// Unique identifer for this compiler event
    pub id: event_id::EventId,

    /// The ID of the [`Event`] which caused this event
    pub parent_id: Option<event_id::EventId>,

    /// The stage of compilation that generated this event
    pub stage: &'a str,

    /// The [`Span`] of input source code that caused this event to occur
    pub input: Span,

    /// A description of the event
    pub msg: Option<Result<V, &'a CompilerError<E>>>,

    /// A stack of currently live [`Event`]s which are being executed by the compiler.
    /// When a new event is created, the top of this stack is the parent. New events
    /// are pushed onto this stack upon creation, and popped off this stack on destruction.
    stack: Option<Rc<RefCell<Vec<event_id::EventId>>>>,
}

impl<'a, V: Writable, E: CompilerDisplay + Debug> Event<'a, V, E> {
    /// Create a new event that does not track event heritage and will not
    /// have a parent ID assigned to it.  This is to support compiler stages
    /// which have not migrated to the new event system and will be deleted
    /// after migration.
    pub fn new_without_parent(
        stage: &'a str,
        input: Span,
        msg: Result<V, &'a CompilerError<E>>,
    ) -> Event<'a, V, E> {
        let id = event_id::EventId::new();
        Event {
            id,
            parent_id: None,
            stage,
            input,
            msg: Some(msg),
            stack: None,
        }
    }

    /// Create a new compiler [`Event`] and assign the result.
    pub fn new_with_result(
        stage: &'a str,
        input: Span,
        msg: Result<V, &'a CompilerError<E>>,
        stack: Rc<RefCell<Vec<event_id::EventId>>>,
    ) -> Event<'a, V, E> {
        let id = event_id::EventId::new();

        // If the stack is not empty, then use the top as the parent id
        let pid = match stack.borrow().last() {
            Some(pid) => Some(*pid),
            None => None,
        };

        // Push the new event on to the stack
        (*stack).borrow_mut().push(id);

        Event {
            id,
            parent_id: pid,
            stage,
            input,
            msg: Some(msg),
            stack: Some(stack),
        }
    }

    /// Create a new compiler [`Event`] without a result.
    pub fn new(
        stage: &'a str,
        input: Span,
        stack: Rc<RefCell<Vec<event_id::EventId>>>,
    ) -> Event<'a, V, E> {
        let id = event_id::EventId::new();

        // If the stack is not empty, then use the top as the parent id
        let pid = match stack.borrow().last() {
            Some(pid) => Some(*pid),
            None => None,
        };

        // Push the new event on to the stack
        (*stack).borrow_mut().push(id);

        Event {
            id,
            parent_id: pid,
            stage,
            input,
            msg: None,
            stack: Some(stack),
        }
    }

    /// Set the Result of this event
    pub fn with_msg(mut self, msg: Result<V, &'a CompilerError<E>>) -> Self {
        self.msg = Some(msg);
        self
    }
}

impl<'a, V: Writable, E: CompilerDisplay + Debug> Drop for Event<'a, V, E> {
    fn drop(&mut self) {
        // Pop the top off the event stack and make sure that the top matches this event
        match self.stack.take() {
            Some(stack) => {
                // pop the top off the stack
                let mut stack = (*stack).borrow_mut();
                match stack.pop() {
                    Some(top_id) if top_id == self.id => (),
                    _ => panic!("Event popped off stack did not match the event being dropped"),
                }
            }
            None => (),
        }
    }
}

impl<'a, V: Writable, E: CompilerDisplay + Debug> Writable for Event<'a, V, E> {
    fn write(&self, w: &dyn Writer) {
        w.start_event();
        w.write_field("id", &self.id);
        match &self.parent_id {
            Some(pid) => w.write_field("parent_id", pid),
            None => (),
        }
        w.write_field("stage", &self.stage);
        w.write_span("source", self.input);
        match &self.msg {
            Some(Ok(msg)) => w.write_field("ok", msg),
            Some(Err(err)) => w.write_field("error", err),
            None => (),
        }
        w.stop_event();
    }
}
