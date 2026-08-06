#![allow(unused)]
use crate::grammar::{
    expr::{Block, LabelId, Stmt},
    Ty,
};
use formality_core::{judgment_fn, term, Set};

/// Computes the control flow that can emerge from a statement.
#[term]
struct ControlFlow {
    can_fall_through: bool,
    breaks: Set<LabelId>,
    continues: Set<LabelId>,
}

impl ControlFlow {
    fn fallthrough() -> Self {
        Self {
            can_fall_through: true,
            breaks: Set::new(),
            continues: Set::new(),
        }
    }

    fn returns() -> Self {
        Self {
            can_fall_through: false,
            breaks: Set::new(),
            continues: Set::new(),
        }
    }

    fn break_to(label: LabelId) -> Self {
        let mut breaks = Set::new();
        breaks.insert(label);

        Self {
            can_fall_through: false,
            breaks,
            continues: Set::new(),
        }
    }

    fn continue_to(label: LabelId) -> Self {
        let mut continues = Set::new();
        continues.insert(label);

        Self {
            can_fall_through: false,
            breaks: Set::new(),
            continues,
        }
    }

    fn then(&self, next: &Self) -> Self {
        if !self.can_fall_through {
            return self.clone();
        }

        Self {
            can_fall_through: next.can_fall_through,
            breaks: self.breaks.union(&next.breaks).cloned().collect(),
            continues: self.continues.union(&next.continues).cloned().collect(),
        }
    }

    fn join(&self, other: &Self) -> Self {
        Self {
            can_fall_through: self.can_fall_through || other.can_fall_through,
            breaks: self.breaks.union(&other.breaks).cloned().collect(),
            continues: self.continues.union(&other.continues).cloned().collect(),
        }
    }

    fn exit_block(&self, label: Option<&LabelId>) -> Self {
        let mut breaks = self.breaks.clone();
        let matching_break = label.is_some_and(|label| breaks.remove(label));

        Self {
            can_fall_through: self.can_fall_through || matching_break,
            breaks,
            continues: self.continues.clone(),
        }
    }

    fn exit_loop(&self, label: Option<&LabelId>) -> Self {
        let mut breaks = self.breaks.clone();
        let mut continues = self.continues.clone();

        let matching_break = if let Some(label) = label {
            let matching_breaks = breaks.remove(label);
            continues.remove(label);

            matching_breaks
        } else {
            false
        };

        Self {
            can_fall_through: matching_break,
            breaks,
            continues,
        }
    }
}

// judgment_fn! {
//   /// Entry point: This answers the question: "Is this function allowed to end?".
//   pub(crate) fn check_fn_returns(
//     output_ty: Ty,
//     block: Block) => () {
//     debug(output_ty, block)

//     (
//       // Rules goes here.
//       // 1. Unit returning function
//       //
//       // (output_ty == ())
//       // ---------------------- ("unit return")
//       // (check_fn_returns(output_ty, block) => ())
//       //
//       // 2. Non-unit returning functions.
//       //
//       // (output_ty != ())
//       // (control_flow_block(block) => flow)
//       // -------------------- ("non-unit returns")
//       // (check_fn_returns(output_ty, block) => ())
//       //
//     )
//   }
// }

// judgment_fn! {
//   /// This answers the question: "What control flow can emerge from this block?".
//   fn control_flow_block(
//     block: Block) => ControlFlow {
//     debug(block)

//     (
//       // Rules goes here.
//     )
//   }
// }

judgment_fn! {
  /// This answers the question: "What control flow can emerge from this one statement.?"
  fn control_flow_stmt(
    stmt: Stmt) => ControlFlow {
    debug(stmt)

    (
      (let flow = ControlFlow::returns())
      --- ("return")
      (control_flow_stmt(Stmt::Return {expr: _ }) => flow)
    )

    (
      (let flow = ControlFlow::break_to(label.clone()))
      --- ("break")
      (control_flow_stmt(Stmt::Break { label }) => flow)
    )

    (
      (let flow = ControlFlow::continue_to(label.clone()))
      --- ("continue")
      (control_flow_stmt(Stmt::Continue { label }) => flow)
    )
  }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn then_ignores_unreachable_breaks() {
        let returned = ControlFlow::returns();
        let unreachable_break = ControlFlow::break_to(LabelId::new("'break"));

        let result = returned.then(&unreachable_break);

        assert!(!result.can_fall_through);
        assert!(result.breaks.is_empty());
        assert!(result.continues.is_empty());
    }

    #[test]
    fn then_preserves_reachable_breaks() {
        let label = LabelId::new("'rb");
        let fallthrough = ControlFlow::fallthrough();
        let break_flow = ControlFlow::break_to(label.clone());

        let result = fallthrough.then(&break_flow);

        assert!(!result.can_fall_through);
        assert!(result.breaks.contains(&label));
        assert!(result.continues.is_empty());
    }

    #[test]
    fn then_ignores_unreachable_continues() {
        let returned = ControlFlow::returns();
        let unreachable_continue = ControlFlow::continue_to(LabelId::new("'cont"));

        let result = returned.then(&unreachable_continue);

        assert!(!result.can_fall_through);
        assert!(result.breaks.is_empty());
        assert!(result.continues.is_empty());
    }

    #[test]
    fn join_unions_break_and_continue_targets_from_both_branches() {
        let break_label = LabelId::new("'bl");
        let break_flow = ControlFlow::break_to(break_label.clone());
        let continue_label = LabelId::new("'cl");
        let continue_flow = ControlFlow::continue_to(continue_label.clone());

        let result = break_flow.join(&continue_flow);

        assert!(!result.can_fall_through);
        assert!(result.breaks.contains(&break_label));
        assert!(result.continues.contains(&continue_label));
    }

    #[test]
    fn join_allows_fallthrough_when_either_branch_falls_through() {
        let label = LabelId::new("'jb");
        let break_flow = ControlFlow::break_to(label.clone());
        let fallthrough = ControlFlow::fallthrough();

        let result = fallthrough.join(&break_flow);

        assert!(result.can_fall_through);
        assert!(result.breaks.contains(&label));
        assert!(result.continues.is_empty());
    }

    #[test]
    fn matching_break_exits_block() {
        let break_label = LabelId::new("'bl");
        let break_flow = ControlFlow::break_to(break_label.clone());

        let result = break_flow.exit_block(Some(&break_label));

        assert!(result.can_fall_through);
        assert!(result.breaks.is_empty());
        assert!(result.continues.is_empty());
    }

    #[test]
    fn continues_propagate_through_block() {
        let block_label = LabelId::new("'inner");
        let continue_label = LabelId::new("'outer");
        let continue_flow = ControlFlow::continue_to(continue_label.clone());

        let result = continue_flow.exit_block(Some(&block_label));

        assert!(!result.can_fall_through);
        assert!(result.breaks.is_empty());
        assert!(result.continues.contains(&continue_label));
    }

    #[test]
    fn matching_break_does_exit_loop() {
        let break_label = LabelId::new("'bl");
        let break_flow = ControlFlow::break_to(break_label.clone());

        let result = break_flow.exit_loop(Some(&break_label));

        assert!(result.can_fall_through);
        assert!(result.breaks.is_empty());
        assert!(result.continues.is_empty());
    }

    #[test]
    fn matching_continue_does_not_exit_loop() {
        let loop_label = LabelId::new("'inner");
        let continue_flow = ControlFlow::continue_to(loop_label.clone());

        let result = continue_flow.exit_loop(Some(&loop_label));

        assert!(!result.can_fall_through);
        assert!(result.breaks.is_empty());
        assert!(result.continues.is_empty());
    }

    #[test]
    fn non_matching_break_propagates_through_blocks() {
        let block_label = LabelId::new("'inner");
        let break_label = LabelId::new("'outer");
        let break_flow = ControlFlow::break_to(break_label.clone());

        let result = break_flow.exit_block(Some(&block_label));

        assert!(!result.can_fall_through);
        assert!(result.breaks.contains(&break_label));
        assert!(result.continues.is_empty());
    }

    #[test]
    fn body_fallthrough_does_not_exit_loop() {
        let block_label = LabelId::new("'loop");
        let fallthrough = ControlFlow::fallthrough();

        let result = fallthrough.exit_loop(Some(&block_label));

        assert!(!result.can_fall_through);
        assert!(result.breaks.is_empty());
        assert!(result.continues.is_empty());
    }

    #[test]
    fn break_targeting_outer_label_propagates_through_inner_loop() {
        let loop_label = LabelId::new("'inner");
        let break_label = LabelId::new("'outer");
        let break_flow = ControlFlow::break_to(break_label.clone());

        let result = break_flow.exit_loop(Some(&loop_label));

        assert!(!result.can_fall_through);
        assert!(result.breaks.contains(&break_label));
        assert!(result.continues.is_empty());
    }

    #[test]
    fn continues_targeting_outer_label_propagates_through_inner_loop() {
        let loop_label = LabelId::new("'inner");
        let continue_label = LabelId::new("'outer");
        let continue_flow = ControlFlow::continue_to(continue_label.clone());

        let result = continue_flow.exit_loop(Some(&loop_label));

        assert!(!result.can_fall_through);
        assert!(result.breaks.is_empty());
        assert!(result.continues.contains(&continue_label));
    }

    #[test]
    fn return_statement_does_not_fall_through() {
        let stmt = Stmt::Return {
            expr: crate::grammar::expr::Expr::True,
        };

        let (flow, _) = control_flow_stmt(stmt).into_singleton().expect("return statement should produce one control flow");

        assert!(!flow.can_fall_through);
        assert!(flow.breaks.is_empty());
        assert!(flow.continues.is_empty());
    }
    
    #[test]
    fn break_statement_records_its_target() {
      let label = LabelId::new("'block");
      let stmt = Stmt::Break {
        label: label.clone()
      };

      let (flow, _) = control_flow_stmt(stmt).into_singleton().expect("break statement should produce one control flow");

      assert!(!flow.can_fall_through);
      assert_eq!(flow.breaks.len(), 1);
      assert!(flow.breaks.contains(&label));
      assert!(flow.continues.is_empty());
    }

    #[test]
    fn continue_statement_records_its_target() {
      let label = LabelId::new("'block");
      let stmt = Stmt::Continue {
        label: label.clone(),
      };

      let (flow, _) = control_flow_stmt(stmt).into_singleton().expect("continue statement should produce one control flow");

      assert!(!flow.can_fall_through);
      assert!(flow.breaks.is_empty());
      assert_eq!(flow.continues.len(), 1);
      assert!(flow.continues.contains(&label));
    }
}
