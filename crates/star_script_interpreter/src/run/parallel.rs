//! `parallel` functions (decision 69): their calls run on the threads of
//! `star_script_runtime::parallel`, as compiled ones do, with copies of
//! the values going there and back. Each thread runs the shared program
//! with values of its own: lists stay on the thread that made them.

use std::cell::{Cell, RefCell};
use std::io;
use std::rc::Rc;
use std::sync::Arc;

use star_script_runtime::{Plain, parallel};

use super::Machine;
use crate::Output;
use crate::error::RunError;
use crate::tree::{Code, FunctionId};
use crate::value::{Value, read};

/// A copy of a value that can travel to another thread: what a `parallel`
/// function takes and gives (never an object, which the analysis
/// refuses there).
#[derive(Debug)]
enum Sent {
    Null,
    Short(i16),
    Int(i32),
    BigInt(i64),
    Float(f32),
    Double(f64),
    Bool(bool),
    String(Arc<str>),
    List(Vec<Sent>),
}

/// A call of a `parallel` function, on its way to a thread.
struct Work {
    code: Arc<Code>,
    function: FunctionId,
    arguments: Vec<Sent>,
}

/// What the work gives back: its result, or the error that stopped it.
///
/// [`Plain::send`] only borrows it, so it hands over what it holds instead
/// of copying a result that nobody else will read.
struct Outcome(Cell<Option<Result<Sent, RunError>>>);

impl Outcome {
    fn new(result: Result<Sent, RunError>) -> Self {
        Self(Cell::new(Some(result)))
    }

    fn into_result(self) -> Result<Sent, RunError> {
        self.0.into_inner().unwrap_or(Err(TAKEN))
    }
}

impl Plain for Outcome {
    type Sent = Result<Sent, RunError>;

    fn send(&self) -> Self::Sent {
        self.0.take().unwrap_or(Err(TAKEN))
    }

    fn receive(sent: Self::Sent) -> Self {
        Self::new(sent)
    }
}

/// The result of a `parallel` function sent twice: the runtime sends it
/// once.
const TAKEN: RunError = RunError::Internal("the result of a `parallel` function sent twice");

impl Machine {
    /// Runs `function` with copies of `arguments` on another thread, and
    /// gives a copy of its result in the first turn after it ends; its
    /// runtime error stops the coroutine that awaits it.
    pub(super) async fn run_parallel(
        &self,
        function: FunctionId,
        arguments: &[Value],
    ) -> Result<Value, RunError> {
        let work = Work {
            code: Arc::clone(&self.code),
            function,
            arguments: arguments.iter().map(send).collect::<Result<_, _>>()?,
        };
        let outcome = parallel::run(work, run_work).await;
        outcome.into_result().map(receive)
    }
}

/// What the thread does: runs the function with values of its own.
fn run_work(work: Work) -> Outcome {
    // A `parallel` function cannot print: the analysis refuses it there.
    let silent: Output = Rc::new(RefCell::new(io::sink()));
    let machine = Machine::new(work.code, silent);
    let arguments = work.arguments.into_iter().map(receive).collect();
    Outcome::new(
        machine
            .call(work.function, arguments)
            .and_then(|result| send(&result)),
    )
}

/// A copy of `value` that can go to another thread; lists are copied
/// whole, so the copy never sees later changes.
fn send(value: &Value) -> Result<Sent, RunError> {
    Ok(match value {
        Value::Null => Sent::Null,
        Value::Short(value) => Sent::Short(*value),
        Value::Int(value) => Sent::Int(*value),
        Value::BigInt(value) => Sent::BigInt(*value),
        Value::Float(value) => Sent::Float(*value),
        Value::Double(value) => Sent::Double(*value),
        Value::Bool(value) => Sent::Bool(*value),
        Value::String(text) => Sent::String(Arc::clone(text)),
        Value::List(elements) => {
            Sent::List(read(elements)?.iter().map(send).collect::<Result<_, _>>()?)
        }
        Value::Object(_) | Value::Color(_) | Value::Node(_) => {
            return Err(RunError::Internal(
                "an object sent to a `parallel` function",
            ));
        }
    })
}

/// The value again, on the thread that receives the copy: its lists are
/// new ones, of this thread.
fn receive(sent: Sent) -> Value {
    match sent {
        Sent::Null => Value::Null,
        Sent::Short(value) => Value::Short(value),
        Sent::Int(value) => Value::Int(value),
        Sent::BigInt(value) => Value::BigInt(value),
        Sent::Float(value) => Value::Float(value),
        Sent::Double(value) => Value::Double(value),
        Sent::Bool(value) => Value::Bool(value),
        Sent::String(text) => Value::String(text),
        Sent::List(elements) => Value::list(elements.into_iter().map(receive).collect()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::write;

    #[test]
    fn copies_of_lists_do_not_see_later_changes() {
        let inner = Value::list(vec![Value::Int(1), Value::Null]);
        let outer = Value::list(vec![inner.clone(), Value::text("a")]);

        let copy = receive(send(&outer).expect("values that can be sent"));
        let Value::List(elements) = &inner else {
            panic!("a list");
        };
        write(elements)
            .expect("a list to change")
            .push(Value::Int(2));

        let expected = Value::list(vec![
            Value::list(vec![Value::Int(1), Value::Null]),
            Value::text("a"),
        ]);
        assert!(copy.star_eq(&expected).expect("comparable values"));
    }

    #[test]
    fn outcomes_hand_over_their_result_once() {
        let outcome = Outcome::new(Ok(Sent::Int(3)));

        let sent = outcome.send();
        let again = Outcome::receive(sent).into_result();

        assert!(matches!(again, Ok(Sent::Int(3))));
        assert!(matches!(outcome.send(), Err(RunError::Internal(_))));
    }
}
