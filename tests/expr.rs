//! The expression language: values, precedence, built-ins, user functions and errors.

use luminesce::expr::Program;
use luminesce::Error;

fn program(defs: &str) -> Program {
    Program::from_source(defs).unwrap_or_else(|e| panic!("definitions failed: {e}"))
}

/// Evaluate `e` at t = `t` against the definitions `defs`.
fn ev_at(defs: &str, e: &str, t: f64) -> f64 {
    let p = program(defs);
    let x = p.compile_expr(e).unwrap_or_else(|err| panic!("`{e}` failed: {err}"));
    x.eval(&p, t)
}

fn ev(e: &str) -> f64 {
    ev_at("", e, 0.0)
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-12
}

fn expr_err(defs: &str, e: &str) -> Error {
    let p = program(defs);
    match p.compile_expr(e) {
        Ok(_) => panic!("`{e}` should have failed"),
        Err(err) => err,
    }
}

fn defs_err(defs: &str) -> Error {
    match Program::from_source(defs) {
        Ok(_) => panic!("definitions should have failed:\n{defs}"),
        Err(err) => err,
    }
}

#[test]
fn numbers_and_the_four_operations() {
    assert_eq!(ev("1 + 2"), 3.0);
    assert_eq!(ev("7 - 10"), -3.0);
    assert_eq!(ev("6 * 7"), 42.0);
    assert_eq!(ev("1 / 4"), 0.25);
    assert_eq!(ev(".5 + 1.25"), 1.75);
}

#[test]
fn precedence_and_associativity() {
    assert_eq!(ev("1 + 2 * 3"), 7.0);
    assert_eq!(ev("(1 + 2) * 3"), 9.0);
    assert_eq!(ev("10 - 4 - 3"), 3.0, "subtraction is left-associative");
    assert_eq!(ev("8 / 4 / 2"), 1.0, "division is left-associative");
    assert_eq!(ev("2 * 3 ^ 2"), 18.0, "power binds tighter than *");
    assert_eq!(ev("2 ^ 3 ^ 2"), 512.0, "power is right-associative");
}

#[test]
fn unary_minus_binds_looser_than_power() {
    assert_eq!(ev("-2 ^ 2"), -4.0, "-2^2 is -(2^2), as in Desmos and maths");
    assert_eq!(ev("(-2) ^ 2"), 4.0);
    assert_eq!(ev("2 ^ -1"), 0.5, "a minus is allowed in the exponent");
    assert_eq!(ev("--3"), 3.0);
    assert_eq!(ev("2 * -3"), -6.0);
    assert_eq!(ev("-3 + 5"), 2.0);
}

#[test]
fn implicit_multiplication() {
    assert_eq!(ev_at("d = 3", "12d + 2", 0.0), 38.0);
    assert_eq!(ev("2(3 + 1)"), 8.0);
    assert_eq!(ev("(1 + 1)(2 + 3)"), 10.0);
    assert_eq!(ev_at("", "2t", 1.5), 3.0);
    assert_eq!(ev_at("", "2t^2", 3.0), 18.0, "2t^2 is 2*(t^2)");
    assert_eq!(ev_at("u = 0.5", "u^2(3 - 2u)", 0.0), 0.5, "u^2(3-2u) is (u^2)*(3-2u)");
    assert_eq!(ev("2 sqrt(9)"), 6.0);
    assert_eq!(ev("1 / 2t"), 0.0, "1/2t is (1/2)*t: implicit multiplication is the same as *");
}

#[test]
fn mod_is_non_negative_for_a_positive_divisor() {
    assert_eq!(ev("mod(13, 12)"), 1.0);
    assert_eq!(ev("mod(-1, 12)"), 11.0);
    assert_eq!(ev("mod(-13, 12)"), 11.0);
    assert_eq!(ev("mod(-24, 12)"), 0.0);
    assert_eq!(ev("mod(-0.5, 1)"), 0.5);
    let tiny = ev("mod(-0.00000000000000001, 12)");
    assert!((0.0..12.0).contains(&tiny), "mod of a tiny negative must stay in [0, 12), got {tiny}");
    assert_eq!(ev("mod(5, -3)"), -1.0, "a negative divisor gives a result in (b, 0], as in Desmos");
    assert!(ev("mod(1, 0)").is_nan(), "mod by zero is NaN, which a render reports loudly");
}

#[test]
fn rounding_built_ins() {
    assert_eq!(ev("floor(-1.5)"), -2.0);
    assert_eq!(ev("ceil(-1.5)"), -1.0);
    assert_eq!(ev("ceil(1.2)"), 2.0);
    assert_eq!(ev("frac(2.75)"), 0.75);
    assert_eq!(ev("frac(-0.25)"), 0.75, "frac(x) = x - floor(x)");
}

#[test]
fn sign_abs_min_max_clamp() {
    assert_eq!(ev("sign(0)"), 0.0);
    assert_eq!(ev("sign(-3)"), -1.0);
    assert_eq!(ev("sign(0.1)"), 1.0);
    assert_eq!(ev("abs(-2.5)"), 2.5);
    assert_eq!(ev("min(3, -1)"), -1.0);
    assert_eq!(ev("max(3, -1)"), 3.0);
    assert_eq!(ev("clamp(5, 0, 1)"), 1.0);
    assert_eq!(ev("clamp(-5, 0, 1)"), 0.0);
    assert_eq!(ev("clamp(0.25, 0, 1)"), 0.25);
}

#[test]
fn trig_sqrt_and_constants() {
    assert!(close(ev("sin(pi / 2)"), 1.0));
    assert!(close(ev("cos(0)"), 1.0));
    assert!(close(ev("cos(tau)"), 1.0));
    assert_eq!(ev("sqrt(9)"), 3.0);
    assert!(ev("sqrt(-1)").is_nan());
}

#[test]
fn user_functions_with_parameters_and_time() {
    let defs = "f(a, b) = a - b\ng(x) = x + t\nk = 2^3\nm = k + 1";
    assert_eq!(ev_at(defs, "f(5, 2)", 0.0), 3.0, "parameters bind in order");
    assert_eq!(ev_at(defs, "g(1)", 2.0), 3.0, "a function can read the time t");
    assert_eq!(ev_at(defs, "m", 0.0), 9.0, "named constants can use each other");
    assert_eq!(ev_at(defs, "f(g(1), k)", 4.0), -3.0);
}

#[test]
fn definitions_may_appear_in_any_order() {
    assert_eq!(ev_at("a = b + 1\nb = 2", "a", 0.0), 3.0);
}

#[test]
fn a_parameter_named_t_shadows_time() {
    assert_eq!(ev_at("f(t) = 2t", "f(5)", 100.0), 10.0);
}

#[test]
fn a_constant_that_uses_t_is_re_evaluated_each_time() {
    let defs = "bar = floor(t / 4)";
    assert_eq!(ev_at(defs, "bar", 3.9), 0.0);
    assert_eq!(ev_at(defs, "bar", 4.0), 1.0);
}

#[test]
fn program_call_evaluates_a_named_function() {
    let p = program("h(d) = floor((12d + 5) / 7)");
    assert_eq!(p.call("h", &[2.0]).unwrap(), 4.0);
    assert!(p.call("nope", &[1.0]).is_err());
    assert!(p.call("h", &[1.0, 2.0]).is_err(), "wrong arity is an error");
}

#[test]
fn comments_and_blank_lines_are_ignored_in_definitions() {
    assert_eq!(ev_at("# a comment\n\na = 1 # trailing\n", "a", 0.0), 1.0);
}

// ---------- errors ----------

#[test]
fn unknown_name_reports_line_and_column() {
    let e = defs_err("x = 1\na = 1 + b");
    assert_eq!((e.line, e.col), (Some(2), Some(9)), "{e}");
    assert!(e.message.contains("unknown name 'b'"), "{e}");
    assert_eq!(e.to_string(), "line 2, column 9: unknown name 'b'");
}

#[test]
fn syntax_error_reports_line_and_column() {
    let e = expr_err("", "1 + * 2");
    assert_eq!((e.line, e.col), (Some(1), Some(5)), "{e}");
    assert!(e.message.contains("found '*'"), "{e}");
}

#[test]
fn missing_closing_paren_is_an_error() {
    let e = expr_err("", "(1 + 2");
    assert!(e.message.contains("')'"), "{e}");
}

#[test]
fn bad_character_is_an_error() {
    let e = expr_err("", "1 $ 2");
    assert_eq!(e.col, Some(3));
    assert!(e.message.contains("'$'"), "{e}");
}

#[test]
fn empty_expression_is_an_error() {
    assert!(expr_err("", "").message.contains("found the end"));
    assert!(expr_err("", "   ").message.contains("found the end"));
}

#[test]
fn trailing_tokens_are_an_error() {
    let e = expr_err("", "1 2");
    assert!(e.message.contains("unexpected number 2"), "{e}");
}

#[test]
fn wrong_arity_is_an_error() {
    assert!(expr_err("", "min(1)").message.contains("takes 2 arguments, got 1"));
    assert!(expr_err("", "sin(1, 2)").message.contains("takes 1 argument, got 2"));
    let e = defs_err("h(d) = d\nx = h(1, 2)");
    assert_eq!(e.line, Some(2));
    assert!(e.message.contains("'h' takes 1 argument, got 2"), "{e}");
}

#[test]
fn a_function_used_without_arguments_is_an_error() {
    assert!(defs_err("f(x) = x\ny = f").message.contains("call it as f(...)"));
    assert!(expr_err("", "sin + 1").message.contains("call it as sin(...)"));
}

#[test]
fn a_value_called_like_a_function_is_an_error() {
    let e = defs_err("f(d) = d(2)");
    assert!(e.message.contains("'d' is not a function"), "{e}");
    assert!(expr_err("", "t(2)").message.contains("'t' is not a function"));
}

#[test]
fn unknown_function_is_an_error() {
    assert!(expr_err("", "tan(1)").message.contains("unknown function 'tan'"));
}

#[test]
fn recursion_is_an_error() {
    assert!(defs_err("f(x) = f(x)").message.contains("f -> f"));
    let e = defs_err("a = b\nb = a");
    assert!(e.message.contains("loop"), "{e}");
}

#[test]
fn redefining_a_built_in_or_t_is_an_error() {
    assert!(defs_err("sin(x) = x").message.contains("built-in"));
    assert!(defs_err("t = 1").message.contains("built-in"));
    assert!(defs_err("pi = 3").message.contains("built-in"));
}

#[test]
fn duplicate_definitions_and_parameters_are_errors() {
    assert!(defs_err("a = 1\na = 2").message.contains("defined twice"));
    assert!(defs_err("f(x, x) = x").message.contains("twice"));
}

#[test]
fn a_constant_that_is_not_finite_is_an_error() {
    let e = defs_err("k = 1 / 0");
    assert!(e.message.contains("not a finite number"), "{e}");
}

#[test]
fn a_malformed_definition_is_an_error() {
    assert!(defs_err("1 = 2").message.contains("expected a definition"));
    assert!(defs_err("f(1) = 2").message.contains("expected a definition"));
    assert!(defs_err("a =").message.contains("nothing after '='"));
    assert!(defs_err("just words").message.contains("expected a definition"));
}

#[test]
fn comparisons_give_one_or_zero() {
    assert_eq!(ev("1 < 2"), 1.0);
    assert_eq!(ev("2 < 2"), 0.0);
    assert_eq!(ev("2 <= 2"), 1.0);
    assert_eq!(ev("3 > 2"), 1.0);
    assert_eq!(ev("2 >= 3"), 0.0);
    assert_eq!(ev("2 == 2"), 1.0);
    assert_eq!(ev("2 != 2"), 0.0);
    assert_eq!(ev("2 != 3"), 1.0);
}

#[test]
fn comparisons_bind_looser_than_arithmetic() {
    assert_eq!(ev("1 + 1 == 2"), 1.0);
    assert_eq!(ev("2 * 3 > 5"), 1.0);
    assert_eq!(ev("-1 < 0"), 1.0);
    // A comparison is a value and can be used inside arithmetic with brackets.
    assert_eq!(ev("3 (2 > 1) + 1"), 4.0);
    assert_eq!(ev_at("", "mod(floor(t), 4) == 3", 3.5), 1.0);
}

#[test]
fn chained_comparisons_are_an_error() {
    let e = expr_err("", "1 < 2 < 3");
    assert!(e.message.contains("chain"), "{e}");
}

#[test]
fn a_comparison_with_nan_is_nan_not_false() {
    assert!(ev("sqrt(-1) < 1").is_nan());
    assert!(ev("sqrt(-1) == sqrt(-1)").is_nan());
}

#[test]
fn if_picks_a_branch_on_a_positive_condition() {
    assert_eq!(ev("if(1, 10, 20)"), 10.0);
    assert_eq!(ev("if(0, 10, 20)"), 20.0);
    assert_eq!(ev("if(-3, 10, 20)"), 20.0);
    assert_eq!(ev_at("f(x) = if(x >= 2, x, -x)", "f(t)", 3.0), 3.0);
    assert_eq!(ev_at("f(x) = if(x >= 2, x, -x)", "f(t)", 1.0), -1.0);
}

#[test]
fn if_evaluates_only_the_branch_it_takes() {
    // The branch not taken would be NaN; it must not leak.
    assert_eq!(ev_at("", "if(t > 0, sqrt(t), sqrt(-1))", 4.0), 2.0);
    assert_eq!(ev_at("", "if(t > 0, sqrt(-1), 7)", 0.0), 7.0);
}

#[test]
fn if_with_a_nan_condition_is_nan() {
    assert!(ev("if(sqrt(-1), 1, 2)").is_nan());
}

#[test]
fn if_is_reserved_and_checks_its_arity() {
    assert!(defs_err("if = 3").message.contains("built-in"));
    assert!(expr_err("", "if(1, 2)").message.contains("takes 3 arguments"));
}

#[test]
fn a_lone_equals_sign_in_an_expression_is_an_error() {
    let e = expr_err("", "1 = 2");
    assert!(e.message.contains("'='"), "{e}");
}

#[test]
fn cached_pure_functions_return_the_right_value_for_every_argument() {
    // `sq` does not use t, so its results may be cached; calls with many different arguments,
    // interleaved and repeated, must each get their own value.
    let p = program("sq(x) = x*x + 1\nsum4(a) = sq(a) + sq(a + 1) + sq(a + 2) + sq(a + 3)");
    let e = p.compile_expr("sum4(floor(t))").unwrap();
    for round in 0..3 {
        for k in [0.0, 5.0, -3.0, 0.0, 1e6, 17.0, 5.0] {
            let want: f64 = (0..4).map(|j| (k + j as f64).powi(2) + 1.0).sum();
            assert_eq!(e.eval(&p, k + 0.5), want, "round {round}, k {k}");
        }
    }
}

#[test]
fn a_function_that_uses_time_is_never_served_stale() {
    // `g` reads t through `f`; the same argument at different times must give different values.
    let p = program("f(x) = x + t\ng(x) = f(x) * 2");
    let e = p.compile_expr("g(1)").unwrap();
    assert_eq!(e.eval(&p, 0.0), 2.0);
    assert_eq!(e.eval(&p, 1.0), 4.0);
    assert_eq!(e.eval(&p, 0.0), 2.0);
}

#[test]
fn nan_and_negative_zero_arguments_are_cached_by_their_bits() {
    let p = program("f(x) = 1 / x");
    let e = p.compile_expr("f(t)").unwrap();
    assert_eq!(e.eval(&p, 0.0), f64::INFINITY);
    assert_eq!(e.eval(&p, -0.0), f64::NEG_INFINITY);
    assert!(e.eval(&p, f64::NAN).is_nan());
    assert_eq!(e.eval(&p, 0.0), f64::INFINITY);
}

#[test]
fn a_function_that_reads_time_only_inside_an_if_branch_is_never_served_stale() {
    let p = program("f(x) = if(x > 0, t, -t)");
    let e = p.compile_expr("f(1) + f(-1)").unwrap();
    assert_eq!(e.eval(&p, 0.0), 0.0);
    let e = p.compile_expr("f(1)").unwrap();
    assert_eq!(e.eval(&p, 2.0), 2.0);
    assert_eq!(e.eval(&p, 3.0), 3.0);
    let e = p.compile_expr("f(-1)").unwrap();
    assert_eq!(e.eval(&p, 5.0), -5.0);
}
