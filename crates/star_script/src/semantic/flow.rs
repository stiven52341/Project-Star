//! Which blocks always leave: by `return`, or also by `break`/`continue`.

use crate::ast::{Block, ElseBranch, IfStatement, Statement, StatementKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Exit {
    /// Only `return` counts: the method ends.
    Return,
    /// `return`, `break` or `continue`: the code after the block is skipped.
    Jump,
}

/// Whether running `block` always ends with a `return`.
pub(super) fn always_returns(block: &Block) -> bool {
    block_exits(block, Exit::Return)
}

/// Whether running `block` never reaches the code after it.
pub(super) fn always_exits(block: &Block) -> bool {
    block_exits(block, Exit::Jump)
}

/// Whether an `else` branch never reaches the code after the `if`.
pub(super) fn branch_exits(branch: &ElseBranch) -> bool {
    else_exits(branch, Exit::Jump)
}

fn block_exits(block: &Block, exit: Exit) -> bool {
    block
        .statements
        .iter()
        .any(|statement| statement_exits(statement, exit))
}

fn statement_exits(statement: &Statement, exit: Exit) -> bool {
    match &statement.kind {
        StatementKind::Return(_) => true,
        StatementKind::Break | StatementKind::Continue => exit == Exit::Jump,
        StatementKind::If(if_statement) => if_exits(if_statement, exit),
        _ => false,
    }
}

fn if_exits(if_statement: &IfStatement, exit: Exit) -> bool {
    block_exits(&if_statement.then_block, exit)
        && if_statement
            .else_branch
            .as_ref()
            .is_some_and(|branch| else_exits(branch, exit))
}

fn else_exits(branch: &ElseBranch, exit: Exit) -> bool {
    match branch {
        ElseBranch::Block(block) => block_exits(block, exit),
        ElseBranch::If(nested) => if_exits(nested, exit),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::MemberKind;
    use crate::parse_source;

    fn method_body(statements: &str) -> Block {
        let source = format!("class T {{ func f() {{\n{statements}\n}} }}");
        let file = parse_source(&source).expect("valid source");
        match &file.classes[0].members[0].kind {
            MemberKind::Method(method) => method.body.clone(),
            _ => unreachable!("the member is a method"),
        }
    }

    #[test]
    fn a_return_anywhere_in_the_block_returns() {
        assert!(always_returns(&method_body("a()\nreturn")));
        assert!(!always_returns(&method_body("a()")));
    }

    #[test]
    fn an_if_returns_only_if_every_branch_does() {
        assert!(always_returns(&method_body(
            "if a { return } else if b { return } else { return }"
        )));
        assert!(!always_returns(&method_body(
            "if a { return } else if b { return }"
        )));
        assert!(!always_returns(&method_body("if a { return }")));
    }

    #[test]
    fn break_exits_but_does_not_return() {
        let body = method_body("break");

        assert!(always_exits(&body));
        assert!(!always_returns(&body));
    }

    #[test]
    fn loops_do_not_count_as_returning() {
        assert!(!always_returns(&method_body("while true { return }")));
    }
}
