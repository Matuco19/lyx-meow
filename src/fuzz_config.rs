// SPDX-License-Identifier: GPL-3.0-only
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

///Root of `lyx-fuzz.toml`, every knob the fuzz run listens to.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FuzzConfig {
    pub run: RunConfig,
    pub r#gen: GenConfig,
    pub stages: StageConfig,
    pub mutate: MutateConfig,
    pub corpus: CorpusConfig,
    pub report: ReportConfig,
}

///Budget and harness behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RunConfig {
    pub cases: u64,
    pub seed: u64,
    pub timeout_ms: u64,
    pub slow_ms: u64,
    pub out_dir: PathBuf,
    pub minimize: bool,
    pub minimize_rounds: usize,
    pub max_cards: usize,
    pub record_slow: bool,
    pub std_path: String,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            cases: 500,
            seed: 1337,
            timeout_ms: 2000,
            slow_ms: 500,
            out_dir: PathBuf::from("fuzz-out"),
            minimize: true,
            minimize_rounds: 24,
            max_cards: 200,
            record_slow: false,
            std_path: String::new(),
        }
    }
}

///Which pipeline stages to pounce on, in order.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StageConfig {
    pub lex: bool,
    pub load: bool,
    pub hir: bool,
    pub ir: bool,
}

impl Default for StageConfig {
    fn default() -> Self {
        Self {
            lex: true,
            load: true,
            hir: true,
            ir: true,
        }
    }
}

///Grammar generator weights and limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GenConfig {
    pub funcs: usize,
    pub stmts_per_func: usize,
    pub max_depth: usize,
    pub components: bool,
    pub generics: bool,
    pub enums: bool,
    pub strings: bool,
    pub unicode: bool,
    pub edge_weight: u32,
}

impl Default for GenConfig {
    fn default() -> Self {
        Self {
            funcs: 4,
            stmts_per_func: 10,
            max_depth: 3,
            components: true,
            generics: true,
            enums: true,
            strings: true,
            unicode: true,
            edge_weight: 30,
        }
    }
}

///Seed mutation weights and operators.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MutateConfig {
    pub enabled: bool,
    pub weight: u32,
    pub max_ops: usize,
    pub byte_flips: bool,
    pub truncate: bool,
    pub duplicate: bool,
    pub swap_lines: bool,
    pub inject_edge: bool,
}

impl Default for MutateConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            weight: 50,
            max_ops: 3,
            byte_flips: true,
            truncate: true,
            duplicate: true,
            swap_lines: true,
            inject_edge: true,
        }
    }
}

///Seed corpus sources.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CorpusConfig {
    pub seeds: Vec<PathBuf>,
    pub max_seed_bytes: usize,
    pub max_seeds: usize,
}

impl Default for CorpusConfig {
    fn default() -> Self {
        Self {
            seeds: vec![PathBuf::from("examples")],
            max_seed_bytes: 8000,
            max_seeds: 64,
        }
    }
}

///Bug card output behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ReportConfig {
    pub save_inputs: bool,
    pub print_ok_every: u64,
}

impl Default for ReportConfig {
    fn default() -> Self {
        Self {
            save_inputs: true,
            print_ok_every: 100,
        }
    }
}

impl FuzzConfig {
    ///Loads explicit path, else `./lyx-fuzz.toml`, else built-in defaults.
    pub fn load(path: Option<&Path>) -> color_eyre::Result<Self> {
        if let Some(p) = path {
            let text = std::fs::read_to_string(p)?;
            return Ok(toml::from_str(&text)?);
        }
        let local = Path::new("lyx-fuzz.toml");
        if local.exists() {
            let text = std::fs::read_to_string(local)?;
            return Ok(toml::from_str(&text)?);
        }
        Ok(Self::default())
    }

    ///CLI flags win over file values.
    pub fn apply_overrides(&mut self, args: &crate::cli::Args) {
        if let Some(cases) = args.fuzz_cases {
            self.run.cases = cases;
        }
        if let Some(seed) = args.fuzz_seed {
            self.run.seed = seed;
        }
        if let Some(out) = args.fuzz_out.clone() {
            self.run.out_dir = out;
        }
    }

    ///Commented starter config for `fuzz --init`.
    pub fn default_toml() -> &'static str {
        r#"# lyx-meow fuzz config — every knob the hairball hunt listens to.
# run with: lyx-meow fuzz [--config=PATH] [--cases=N] [--seed=N] [--out=DIR]

[run]
cases = 500          # total cases to pounce on
seed = 1337          # deterministic stream, same seed = same hunt
timeout_ms = 2000    # per-case budget, overruns become hang cards
slow_ms = 500        # cases slower than this are counted as slow
out_dir = "fuzz-out" # cards land in <out_dir>/cards
minimize = true      # shrink panic reproducers line by line
minimize_rounds = 24 # max shrink attempts per panic card
max_cards = 200      # stop saving new cards past this (counts continue)
record_slow = false  # also file a card for slow (non-hang) cases
std_path = ""        # slynx std dir, empty inherits STD_PATH or ~/.slynx/std

[stages]             # pipeline stages to run in order, first panic wins
lex = true           # lexer tokenization
load = true          # parsing + module loading
hir = true           # HIR + typecheck + ownership + monomorphize
ir = true            # codegen lowering to SlynxIR

[gen]                # grammar generator, valid-ish programs reaching deep stages
funcs = 4            # helper funcs per generated program
stmts_per_func = 10  # statements per func body
max_depth = 3        # expression nesting depth
components = true    # emit component decls
generics = true      # emit generic funcs with explicit type args
enums = true         # emit enum decls + matches
strings = true       # emit string literals with escapes
unicode = true       # emit non-ascii idents and strings
edge_weight = 30     # 0-100 chance per case to blend curated edge snippets

[mutate]             # seed mutation, chaos on real programs
enabled = true
weight = 50          # 0-100 chance per case to mutate a corpus seed
max_ops = 3          # mutation ops stacked per case
byte_flips = true    # random byte swaps
truncate = true      # chop the tail mid-token
duplicate = true     # repeat a random span
swap_lines = true    # shuffle two lines
inject_edge = true   # splice a curated edge snippet inside

[corpus]             # seed programs, files or dirs with .syx/.slx/.slynx
seeds = ["examples"]
max_seed_bytes = 8000
max_seeds = 64

[report]
save_inputs = true   # write .syx reproducers next to cards
print_ok_every = 100 # progress pawprint cadence, 0 disables
"#
    }
}

#[cfg(test)]
mod tests {
    use super::FuzzConfig;

    #[test]
    fn default_toml_parses_back() {
        let cfg: FuzzConfig =
            toml::from_str(FuzzConfig::default_toml()).expect("starter config should parse");
        assert_eq!(cfg.run.cases, 500);
        assert!(cfg.stages.hir);
    }

    #[test]
    fn partial_config_fills_defaults() {
        let cfg: FuzzConfig =
            toml::from_str("[run]\ncases = 7\n").expect("partial config should parse");
        assert_eq!(cfg.run.cases, 7);
        assert_eq!(cfg.run.seed, 1337);
        assert!(cfg.mutate.enabled);
    }
}
