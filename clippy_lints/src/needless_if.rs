use clippy_utils::{diagnostics::span_lint_and_sugg, is_from_proc_macro, source::snippet_with_applicability};
use rustc_errors::Applicability;
use rustc_hir::{
    intravisit::{walk_expr, Visitor},
    Expr, ExprKind, Node,
};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::lint::in_external_macro;
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for empty `if` branches with no else branch.
    ///
    /// ### Why is this bad?
    /// It can be entirely omitted, and often the condition too.
    ///
    /// ### Known issues
    /// This will usually only suggest to remove the `if` statement, not the condition. Other lints
    /// such as `no_effect` will take care of removing the condition if it's unnecessary.
    ///
    /// ### Example
    /// ```rust,ignore
    /// if really_expensive_condition(&i) {}
    /// if really_expensive_condition_with_side_effects(&mut i) {}
    /// ```
    /// Use instead:
    /// ```rust,ignore
    /// // <omitted>
    /// really_expensive_condition_with_side_effects(&mut i);
    /// ```
    #[clippy::version = "1.72.0"]
    pub NEEDLESS_IF,
    complexity,
    "checks for empty if branches"
}
declare_lint_pass!(NeedlessIf => [NEEDLESS_IF]);

impl LateLintPass<'_> for NeedlessIf {
    fn check_expr<'tcx>(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
        if let ExprKind::If(if_expr, block, else_expr) = &expr.kind
            && let ExprKind::Block(block, ..) = block.kind
            && block.stmts.is_empty()
            && block.expr.is_none()
            && else_expr.is_none()
            && !in_external_macro(cx.sess(), expr.span)
        {
            // Ignore `else if`
            if let Some(parent_id) = cx.tcx.hir().opt_parent_id(expr.hir_id)
                && let Some(Node::Expr(Expr {
                    kind: ExprKind::If(_, _, Some(else_expr)),
                    ..
                })) = cx.tcx.hir().find(parent_id)
                && else_expr.hir_id == expr.hir_id
            {
                return;
            }

            if is_any_if_let(if_expr) || is_from_proc_macro(cx, expr) {
                return;
            }

            let mut app = Applicability::MachineApplicable;
            let snippet = snippet_with_applicability(cx, if_expr.span, "{ ... }", &mut app);

            span_lint_and_sugg(
                cx,
                NEEDLESS_IF,
                expr.span,
                "this `if` branch is empty",
                "you can remove it",
                if if_expr.can_have_side_effects() {
                    format!("{snippet};")
                } else {
                    String::new()
                },
                app,
            );
        }
    }
}

/// Returns true if any `Expr` contained within this `Expr` is a `Let`, else false.
///
/// Really wish `Expr` had a `walk` method...
fn is_any_if_let(expr: &Expr<'_>) -> bool {
    let mut v = IsAnyLetVisitor(false);

    v.visit_expr(expr);
    v.0
}

struct IsAnyLetVisitor(bool);

impl Visitor<'_> for IsAnyLetVisitor {
    fn visit_expr(&mut self, expr: &Expr<'_>) {
        if matches!(expr.kind, ExprKind::Let(..)) {
            self.0 = true;
        } else {
            walk_expr(self, expr);
        }
    }
}