// SPDX-License-Identifier: GPL-3.0-only
use crate::fuzz_config::{FuzzConfig, GenConfig, MutateConfig};

///Tiny xorshift64 stream, deterministic per seed, no extra deps.
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_add(0x9e37_79b9_7f4a_7c15).max(1),
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x.max(1);
        x
    }

    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        (self.next_u64() % n as u64) as usize
    }

    pub fn chance(&mut self, weight_0_100: u32) -> bool {
        self.below(100) < weight_0_100.min(100) as usize
    }

    pub fn pick<'a>(&mut self, items: &'a [&'a str]) -> &'a str {
        items[self.below(items.len())]
    }
}

// curated nasties: malformed numbers, unterminated liquids, deep nesting bait
pub const EDGE_POOL: [&str; 24] = [
    "4..0",
    "1._0",
    "1_.0",
    "99999999999999999999999999",
    "0x",
    "0b102",
    "\"unterminated",
    "/* unterminated",
    "//",
    "?",
    "func",
    "{{{{{{",
    "}}}}}}",
    "((((((1))))))",
    "let _ = _;",
    "let x: []int = {};",
    "β",
    "\"çãõ\\n\\t\\u{0}\"",
    "2_000_0_0_0",
    ".0",
    "pair.0.age",
    "a matches",
    "\0",
    "func main(): void { let a = (1, ); }",
];

const INTS: [&str; 8] = ["0", "1", "42", "0x2A", "0b0110", "0o17", "2_000", "1000000"];
const STRINGS: [&str; 5] = ["\"meow\"", "\"a\\nb\"", "\"q\\\"q\"", "\"\"", "\"çãõ\""];
const IDENTS: [&str; 8] = ["a", "x", "val", "count", "pessoa", "age", "tmp", "n"];
const UNICODE_IDENTS: [&str; 4] = ["β", "çã", "miau_β", "aõ"];
const BINOPS: [&str; 8] = ["+", "-", "*", "/", "<", ">", "==", "&&"];

///Generates one valid-ish slynx program reaching deep stages.
pub fn generate(rng: &mut Rng, gc: &GenConfig) -> String {
    let mut out = String::new();
    let mut helpers: Vec<String> = Vec::new();

    // one enum per program so uses can actually resolve
    let enum_name = if gc.enums && rng.chance(50) {
        let name = format!("E{}", rng.below(100));
        out.push_str(&format!("enum {name} {{\n    Raw,\n    Val\n}}\n\n"));
        Some(name)
    } else {
        None
    };
    if gc.components && rng.chance(50) {
        out.push_str(&format!("component C{} {{\n}}\n\n", rng.below(100)));
    }
    let n_funcs = 1 + rng.below(gc.funcs.max(1));
    for i in 0..n_funcs {
        let name = format!("h{i}_{}", rng.below(1000));
        helpers.push(name.clone());
        if gc.generics && rng.chance(30) {
            out.push_str(&format!("func {name}<T>(x: T): T {{\n    x\n}}\n\n"));
        } else {
            let ret = rng.pick(&["int", "bool"]);
            out.push_str(&format!("func {name}(a: int, b: int): {ret} {{\n"));
            out.push_str(&func_body(rng, gc, &helpers, enum_name.as_deref()));
            out.push_str("}\n\n");
        }
    }
    out.push_str("func main(): void {\n");
    out.push_str(&func_body(rng, gc, &helpers, enum_name.as_deref()));
    out.push_str("}\n");
    out
}

// body of lets, ifs and whiles with trailing expression flavor
fn func_body(rng: &mut Rng, gc: &GenConfig, helpers: &[String], enum_name: Option<&str>) -> String {
    let mut out = String::new();
    // seed known int locals so calls and binops often typecheck
    out.push_str("    let a = 1;\n    let b = 2;\n");
    let n = 1 + rng.below(gc.stmts_per_func.max(1));
    let mut locals: Vec<String> = vec!["a".to_string(), "b".to_string()];
    for i in 0..n {
        let v = format!("v{i}");
        match rng.below(5) {
            0 => {
                let mutable = if rng.chance(40) { "mut " } else { "" };
                out.push_str(&format!(
                    "    let {mutable}{v} = {};\n",
                    expr(rng, gc, &locals, helpers, enum_name, gc.max_depth)
                ));
                locals.push(v);
            }
            1 if !locals.is_empty() => {
                let target = locals[rng.below(locals.len())].clone();
                out.push_str(&format!(
                    "    {target} = {};\n",
                    expr(rng, gc, &locals, helpers, enum_name, gc.max_depth)
                ));
            }
            2 => {
                out.push_str(&format!(
                    "    if {} {{\n        {};\n    }} else {{\n        {};\n    }};\n",
                    expr(rng, gc, &locals, helpers, enum_name, 1),
                    expr(rng, gc, &locals, helpers, enum_name, 1),
                    expr(rng, gc, &locals, helpers, enum_name, 1),
                ));
            }
            3 => {
                out.push_str(&format!(
                    "    while {} {{\n        {};\n    }}\n",
                    expr(rng, gc, &locals, helpers, enum_name, 1),
                    expr(rng, gc, &locals, helpers, enum_name, 1),
                ));
            }
            _ => {
                out.push_str(&format!(
                    "    {};\n",
                    expr(rng, gc, &locals, helpers, enum_name, gc.max_depth)
                ));
            }
        }
    }
    out
}

// depth-limited expression mixing literals, tuples, arrays, calls and binops
fn expr(
    rng: &mut Rng,
    gc: &GenConfig,
    locals: &[String],
    helpers: &[String],
    enum_name: Option<&str>,
    depth: usize,
) -> String {
    if depth == 0 {
        return leaf(rng, gc, locals);
    }
    match rng.below(8) {
        0 => leaf(rng, gc, locals),
        1 => format!(
            "({}, {})",
            expr(rng, gc, locals, helpers, enum_name, depth - 1),
            expr(rng, gc, locals, helpers, enum_name, depth - 1)
        ),
        2 => format!(
            "[{}, {}]",
            expr(rng, gc, locals, helpers, enum_name, depth - 1),
            expr(rng, gc, locals, helpers, enum_name, depth - 1)
        ),
        3 if !helpers.is_empty() => {
            let h = helpers[rng.below(helpers.len())].clone();
            format!(
                "{h}({}, {})",
                expr(rng, gc, locals, helpers, enum_name, 0),
                expr(rng, gc, locals, helpers, enum_name, 0)
            )
        }
        4 => format!(
            "({} {} {})",
            expr(rng, gc, locals, helpers, enum_name, depth - 1),
            rng.pick(&BINOPS),
            expr(rng, gc, locals, helpers, enum_name, depth - 1)
        ),
        5 => format!(
            "{}.{}",
            expr(rng, gc, locals, helpers, enum_name, depth - 1),
            rng.below(3)
        ),
        6 if enum_name.is_some() && rng.chance(40) => {
            format!("{}.Raw", enum_name.expect("enum name checked above"))
        }
        _ => leaf(rng, gc, locals),
    }
}

// atoms: numbers, bools, strings, idents
fn leaf(rng: &mut Rng, gc: &GenConfig, locals: &[String]) -> String {
    match rng.below(6) {
        0 => rng.pick(&INTS).to_string(),
        1 => rng.pick(&["true", "false"]).to_string(),
        2 if gc.strings => rng.pick(&STRINGS).to_string(),
        3 if !locals.is_empty() => locals[rng.below(locals.len())].clone(),
        4 if gc.unicode && rng.chance(30) => rng.pick(&UNICODE_IDENTS).to_string(),
        _ => rng.pick(&IDENTS).to_string(),
    }
}

///Picks the source for case `i`: mutated seed, edge blend, or fresh program.
pub fn case_source(
    rng: &mut Rng,
    cfg: &FuzzConfig,
    corpus: &[String],
    case_no: u64,
) -> (String, String) {
    let _ = case_no;
    if cfg.mutate.enabled && !corpus.is_empty() && rng.chance(cfg.mutate.weight) {
        let seed = corpus[rng.below(corpus.len())].clone();
        return ("mutant".to_string(), mutate(rng, &seed, &cfg.mutate));
    }
    if rng.chance(cfg.r#gen.edge_weight) {
        let mut src = generate(rng, &cfg.r#gen);
        src.push('\n');
        src.push_str(rng.pick(&EDGE_POOL));
        src.push('\n');
        return ("edge-blend".to_string(), src);
    }
    ("generated".to_string(), generate(rng, &cfg.r#gen))
}

///Stacks up to `max_ops` mutation operators onto a seed.
pub fn mutate(rng: &mut Rng, src: &str, mc: &MutateConfig) -> String {
    let mut out = src.to_string();
    let ops = 1 + rng.below(mc.max_ops.max(1));
    for _ in 0..ops {
        let choices: Vec<usize> = [
            mc.byte_flips,
            mc.truncate,
            mc.duplicate,
            mc.swap_lines,
            mc.inject_edge,
        ]
        .iter()
        .enumerate()
        .filter_map(|(i, on)| on.then_some(i))
        .collect();
        if choices.is_empty() {
            break;
        }
        match choices[rng.below(choices.len())] {
            0 => op_byte_flip(rng, &mut out),
            1 => op_truncate(rng, &mut out),
            2 => op_duplicate(rng, &mut out),
            3 => op_swap_lines(rng, &mut out),
            _ => {
                let edge: &str = rng.pick(&EDGE_POOL);
                let at = rng.below(out.len() + 1);
                out.insert_str(at.min(out.len()), edge);
            }
        }
        if out.len() > 64_000 {
            out.truncate(64_000);
        }
    }
    out
}

// swaps one byte for a keyboard-adjacent cousin
fn op_byte_flip(rng: &mut Rng, out: &mut String) {
    if out.is_empty() {
        return;
    }
    // stay on char boundaries to keep strings valid utf8
    let idx = out
        .char_indices()
        .nth(rng.below(out.chars().count()))
        .map(|(i, _)| i)
        .unwrap_or(0);
    let cousins = [
        "{", "}", "(", ")", ".", ",", ":", ";", "?", "@", "0", "\"", " ",
    ];
    let at = idx.min(out.len());
    out.insert_str(at, rng.pick(&cousins));
}

// chops the tail, often mid-token
fn op_truncate(rng: &mut Rng, out: &mut String) {
    if out.is_empty() {
        return;
    }
    let mut len = rng.below(out.len() + 1);
    while len > 0 && !out.is_char_boundary(len) {
        len -= 1;
    }
    out.truncate(len);
}

// repeats a random span to stress nesting and arity checks
fn op_duplicate(rng: &mut Rng, out: &mut String) {
    if out.is_empty() {
        return;
    }
    let mut start = rng.below(out.len());
    while !out.is_char_boundary(start) {
        start = start.saturating_sub(1);
    }
    let mut end = (start + rng.below(out.len().saturating_sub(start) + 1)).min(out.len());
    while !out.is_char_boundary(end) {
        end = end.saturating_sub(1).max(start);
    }
    if end > start {
        let span = out[start..end].to_string();
        out.insert_str(end.min(out.len()), &span);
    }
}

// shuffles two lines to break declaration order
fn op_swap_lines(rng: &mut Rng, out: &mut String) {
    let mut lines: Vec<&str> = out.lines().collect();
    if lines.len() < 2 {
        return;
    }
    let a = rng.below(lines.len());
    let b = rng.below(lines.len());
    lines.swap(a, b);
    *out = lines.join("\n");
}

#[cfg(test)]
mod tests {
    use super::{Rng, case_source, generate, mutate};
    use crate::fuzz_config::FuzzConfig;

    #[test]
    fn same_seed_same_program() {
        let gc = FuzzConfig::default().r#gen;
        let a = generate(&mut Rng::new(7), &gc);
        let b = generate(&mut Rng::new(7), &gc);
        assert_eq!(a, b);
    }

    #[test]
    fn generated_programs_usually_lex() {
        let mut gc = FuzzConfig::default().r#gen;
        // unicode idents are lex errors by design, exercised elsewhere
        gc.unicode = false;
        let mut ok = 0;
        for seed in 0..10 {
            let src = generate(&mut Rng::new(seed), &gc);
            if slynx::slynx_lexer::Lexer::tokenize(&src).is_ok() {
                ok += 1;
            }
        }
        assert!(ok >= 8, "generator should mostly emit lexable code");
    }

    #[test]
    fn mutate_stays_bounded() {
        let mc = FuzzConfig::default().mutate;
        let out = mutate(&mut Rng::new(3), "func main(): void {}", &mc);
        assert!(out.len() <= 64_000);
    }

    #[test]
    fn case_source_covers_origins() {
        let cfg = FuzzConfig::default();
        let corpus = vec!["func main(): void {}".to_string()];
        let mut kinds = std::collections::HashSet::new();
        for i in 0..50 {
            let (origin, _) = case_source(&mut Rng::new(i), &cfg, &corpus, i);
            kinds.insert(origin);
        }
        assert!(kinds.len() > 1);
    }
}
