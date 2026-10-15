// SPDX-License-Identifier: GPL-3.0-only
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use crate::{
    cli::Args,
    fuzz_config::FuzzConfig,
    fuzz_gen::{Rng, case_source},
    theme,
};

///Pipeline stage where a case blew up.
pub const STAGE_LEX: &str = "lex";
pub const STAGE_LOAD: &str = "load";
pub const STAGE_HIR: &str = "hir";
pub const STAGE_IR: &str = "ir";

///How a case failed the hunt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BugKind {
    Panic,
    Hang,
    Slow,
}

impl BugKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Panic => "panic",
            Self::Hang => "hang",
            Self::Slow => "slow",
        }
    }
}

///One filed hairball with everything needed to reproduce it.
#[derive(Debug, Clone)]
pub struct BugCard {
    pub id: String,
    pub title: String,
    pub stage: String,
    pub kind: BugKind,
    pub case_no: u64,
    pub seed: u64,
    pub origin: String,
    pub occurrences: usize,
    pub input: String,
    pub minimized: Option<String>,
    pub message: String,
    pub ms: u128,
}

///Outcome of a single case.
enum CaseOutcome {
    Clean,
    Diagnostic,
    Panic {
        stage: String,
        message: String,
        ms: u128,
    },
    Hang {
        stage: String,
        ms: u128,
    },
    Slow {
        ms: u128,
    },
}

///Worker heartbeat, lets the driver name the hung stage.
enum Msg {
    Progress(&'static str),
    Done(CaseOutcome),
}

///Totals for the summary card.
#[derive(Debug, Default)]
pub struct FuzzSummary {
    pub cases: u64,
    pub clean: u64,
    pub diagnostics: u64,
    pub panics: u64,
    pub hangs: u64,
    pub slows: u64,
    pub reached_lex: u64,
    pub reached_load: u64,
    pub reached_hir: u64,
    pub reached_ir: u64,
    pub harness_skips: u64,
    pub cards: Vec<BugCard>,
    pub out_dir: PathBuf,
}

///Entry point for `lyx-meow fuzz`, handles `--init` and config resolution.
pub fn run(args: &Args) -> color_eyre::Result<()> {
    if args.fuzz_init {
        let target = args
            .fuzz_config
            .clone()
            .unwrap_or_else(|| PathBuf::from("lyx-fuzz.toml"));
        if target.exists() {
            println!(
                "{} already purring at {}",
                theme::green("="),
                target.display()
            );
            return Ok(());
        }
        std::fs::write(&target, FuzzConfig::default_toml())?;
        println!(
            "{} starter config kneaded at {}",
            theme::bright("✓ purrfect"),
            target.display()
        );
        println!("  tweak it, then run: lyx-meow fuzz");
        return Ok(());
    }
    let mut cfg = FuzzConfig::load(args.fuzz_config.as_deref())?;
    cfg.apply_overrides(args);
    let summary = run_with_config(&cfg)?;
    print_summary(&summary);
    Ok(())
}

///Runs the full hunt headlessly, used by tests too.
pub fn run_with_config(cfg: &FuzzConfig) -> color_eyre::Result<FuzzSummary> {
    if !cfg.run.std_path.is_empty() {
        // driver is single-threaded here, workers spawn after
        unsafe {
            std::env::set_var("STD_PATH", &cfg.run.std_path);
        }
    }
    let cards_dir = cfg.run.out_dir.join("cards");
    std::fs::create_dir_all(&cards_dir)?;

    let corpus = load_corpus(cfg);
    let mut rng = Rng::new(cfg.run.seed);
    let mut summary = FuzzSummary {
        out_dir: cfg.run.out_dir.clone(),
        ..FuzzSummary::default()
    };
    let mut seen: HashMap<String, usize> = HashMap::new();

    println!(
        "{} sniffing {} cases (seed {})",
        theme::logo(),
        cfg.run.cases,
        cfg.run.seed
    );
    for case_no in 0..cfg.run.cases {
        summary.cases += 1;
        // the hunt must survive its own generator: a harness panic skips loudly
        let generated = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            case_source(&mut rng, cfg, &corpus, case_no)
        }));
        let (origin, src) = match generated {
            Ok(pair) => pair,
            Err(payload) => {
                summary.harness_skips += 1;
                if summary.harness_skips <= 3 {
                    println!(
                        "  {} harness hairball on case #{case_no}: {}",
                        theme::green("!"),
                        panic_message(payload).lines().next().unwrap_or("unknown")
                    );
                }
                continue;
            }
        };
        let (outcome, reached) = run_case(&src, cfg);
        for stage in &reached {
            match *stage {
                STAGE_LEX => summary.reached_lex += 1,
                STAGE_LOAD => summary.reached_load += 1,
                STAGE_HIR => summary.reached_hir += 1,
                STAGE_IR => summary.reached_ir += 1,
                _ => {}
            }
        }
        match outcome {
            CaseOutcome::Clean => summary.clean += 1,
            CaseOutcome::Diagnostic => summary.diagnostics += 1,
            CaseOutcome::Slow { ms } => {
                summary.slows += 1;
                if cfg.run.record_slow {
                    file_card(
                        &mut summary,
                        &mut seen,
                        cfg,
                        case_no,
                        &origin,
                        &src,
                        deepest_effective(&cfg.stages),
                        BugKind::Slow,
                        &format!("case slower than slow_ms ({ms}ms)"),
                        ms,
                    );
                }
            }
            CaseOutcome::Panic { stage, message, ms } => {
                summary.panics += 1;
                file_card(
                    &mut summary,
                    &mut seen,
                    cfg,
                    case_no,
                    &origin,
                    &src,
                    &stage,
                    BugKind::Panic,
                    &message,
                    ms,
                );
            }
            CaseOutcome::Hang { stage, ms } => {
                summary.hangs += 1;
                let last = reached.last().copied().unwrap_or("nothing");
                file_card(
                    &mut summary,
                    &mut seen,
                    cfg,
                    case_no,
                    &origin,
                    &src,
                    &stage,
                    BugKind::Hang,
                    &format!("no answer within {ms}ms, last completed: {last}"),
                    ms,
                );
            }
        }
        if cfg.report.print_ok_every > 0 && (case_no + 1) % cfg.report.print_ok_every == 0 {
            println!(
                "  🐾 pounced {}/{} · hairballs {}",
                case_no + 1,
                cfg.run.cases,
                summary.cards.len()
            );
        }
    }
    write_summary_file(&summary, cfg)?;
    Ok(summary)
}

// files one card: dedupes, minimizes panics, saves + prints
#[allow(clippy::too_many_arguments)]
fn file_card(
    summary: &mut FuzzSummary,
    seen: &mut HashMap<String, usize>,
    cfg: &FuzzConfig,
    case_no: u64,
    origin: &str,
    src: &str,
    stage: &str,
    kind: BugKind,
    message: &str,
    ms: u128,
) {
    let key = dedup_key(stage, kind, message);
    if let Some(idx) = seen.get(&key) {
        summary.cards[*idx].occurrences += 1;
        return;
    }
    if summary.cards.len() >= cfg.run.max_cards {
        return;
    }
    let n = summary.cards.len() + 1;
    let minimized = if cfg.run.minimize && kind == BugKind::Panic {
        minimize(src, cfg, &key)
    } else {
        None
    };
    let card = BugCard {
        id: format!("FUZZ-{n:04}"),
        title: format!("{} in {stage}", kind.as_str()),
        stage: stage.to_string(),
        kind,
        case_no,
        seed: cfg.run.seed,
        origin: origin.to_string(),
        occurrences: 1,
        input: src.to_string(),
        minimized,
        message: message.lines().take(12).collect::<Vec<_>>().join("\n"),
        ms,
    };
    print_card(&card, &cfg.run.out_dir);
    save_card(&card, cfg);
    seen.insert(key, summary.cards.len());
    summary.cards.push(card);
}

// runs one case in a worker with timeout, returns completed stages too
fn run_case(src: &str, cfg: &FuzzConfig) -> (CaseOutcome, Vec<&'static str>) {
    let (tx, rx) = std::sync::mpsc::channel();
    let input = src.to_string();
    let stages = cfg.stages.clone();
    let budget = Duration::from_millis(cfg.run.timeout_ms.max(1));
    let slow = cfg.run.slow_ms;
    let root = cfg.run.out_dir.join("case.syx");
    std::thread::spawn(move || {
        let beat = |stage: &'static str| {
            let _ = tx.send(Msg::Progress(stage));
        };
        let outcome = run_stages(&input, &stages, &root, slow, &beat);
        let _ = tx.send(Msg::Done(outcome));
    });
    let mut reached: Vec<&'static str> = Vec::new();
    let deadline = Instant::now() + budget;
    loop {
        // absolute deadline, heartbeats never extend it
        let left = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(left) {
            Ok(Msg::Progress(stage)) => {
                if !reached.contains(&stage) {
                    reached.push(stage);
                }
            }
            Ok(Msg::Done(outcome)) => return (outcome, reached),
            Err(_) => {
                let hung = hung_stage(&cfg.stages, &reached);
                return (
                    CaseOutcome::Hang {
                        stage: hung.to_string(),
                        ms: cfg.run.timeout_ms as u128,
                    },
                    reached,
                );
            }
        }
    }
}

// first enabled stage with no heartbeat, the one the worker died in
fn hung_stage(stages: &crate::fuzz_config::StageConfig, reached: &[&'static str]) -> &'static str {
    for stage in effective_stages(stages) {
        if !reached.contains(&stage) {
            return stage;
        }
    }
    effective_stages(stages).last().copied().unwrap_or("run")
}

// enabled stages in pipeline order, hir/ir imply their prerequisites
fn effective_stages(stages: &crate::fuzz_config::StageConfig) -> Vec<&'static str> {
    let mut out = Vec::new();
    if stages.lex {
        out.push(STAGE_LEX);
    }
    if stages.load || stages.hir || stages.ir {
        out.push(STAGE_LOAD);
    }
    if stages.hir || stages.ir {
        out.push(STAGE_HIR);
    }
    if stages.ir {
        out.push(STAGE_IR);
    }
    out
}

// staged pipeline with per-stage panic catch, attribution is exact
fn run_stages(
    src: &str,
    stages: &crate::fuzz_config::StageConfig,
    root: &Path,
    slow: u64,
    beat: &dyn Fn(&'static str),
) -> CaseOutcome {
    let start = Instant::now();
    let ms = || start.elapsed().as_millis();

    if stages.lex {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            slynx::slynx_lexer::Lexer::tokenize(src).map_err(|e| format!("{e:?}"))
        })) {
            Err(payload) => {
                return CaseOutcome::Panic {
                    stage: STAGE_LEX.to_string(),
                    message: panic_message(payload),
                    ms: ms(),
                };
            }
            Ok(Err(_)) => return CaseOutcome::Diagnostic,
            Ok(Ok(_)) => beat(STAGE_LEX),
        }
    }

    // remaining stages share one context, in-memory source
    let ctx = slynx::SlynxContext::from_source(src.to_string(), root);
    let effective = effective_stages(stages);

    let modules = if effective.contains(&STAGE_LOAD) {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            ctx.load_modules().map_err(|e| format!("{e:?}"))
        })) {
            Err(payload) => {
                return CaseOutcome::Panic {
                    stage: STAGE_LOAD.to_string(),
                    message: panic_message(payload),
                    ms: ms(),
                };
            }
            Ok(Err(_)) => return CaseOutcome::Diagnostic,
            Ok(Ok(modules)) => {
                beat(STAGE_LOAD);
                Some(modules)
            }
        }
    } else {
        None
    };

    // hir implies load, so modules is always present here
    let lowered = if effective.contains(&STAGE_HIR) {
        let Some(modules) = modules.as_ref() else {
            return CaseOutcome::Diagnostic;
        };
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            ctx.build_hir(modules).map_err(|e| format!("{e:?}"))
        })) {
            Err(payload) => {
                return CaseOutcome::Panic {
                    stage: STAGE_HIR.to_string(),
                    message: panic_message(payload),
                    ms: ms(),
                };
            }
            Ok(Err(_)) => return CaseOutcome::Diagnostic,
            Ok(Ok(lowered)) => {
                beat(STAGE_HIR);
                Some(lowered)
            }
        }
    } else {
        None
    };

    if effective.contains(&STAGE_IR) {
        // ir implies hir, so lowered artifacts are always present here
        let Some((hir, deadcode, ownership)) = lowered else {
            return CaseOutcome::Diagnostic;
        };
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            ctx.build_ir(hir, deadcode, ownership)
                .map_err(|e| format!("{e:?}"))
        })) {
            Err(payload) => {
                return CaseOutcome::Panic {
                    stage: STAGE_IR.to_string(),
                    message: panic_message(payload),
                    ms: ms(),
                };
            }
            Ok(Err(_)) => return CaseOutcome::Diagnostic,
            Ok(Ok(_)) => beat(STAGE_IR),
        }
    }

    if effective.is_empty() {
        return CaseOutcome::Clean;
    }
    let elapsed = ms();
    if slow > 0 && elapsed > slow as u128 {
        CaseOutcome::Slow { ms: elapsed }
    } else {
        CaseOutcome::Clean
    }
}

// line-based shrinker, keeps the smallest input with the same dedup key
fn minimize(src: &str, cfg: &FuzzConfig, key: &str) -> Option<String> {
    let mut best = src.to_string();
    if best.lines().count() < 2 {
        return None;
    }
    let mut rounds = 0;
    let mut improved = true;
    while improved && rounds < cfg.run.minimize_rounds {
        improved = false;
        rounds += 1;
        let lines: Vec<String> = best.lines().map(str::to_string).collect();
        // try dropping single lines first, then pairs
        let mut next: Option<String> = None;
        for width in [1, 2] {
            let mut i = 0;
            while i + width <= lines.len() && rounds < cfg.run.minimize_rounds {
                rounds += 1;
                let mut candidate = lines.clone();
                candidate.drain(i..i + width);
                let text = candidate.join("\n");
                if !text.trim().is_empty() && reproduces(&text, cfg, key) {
                    next = Some(text);
                    break;
                }
                i += 1;
            }
            if next.is_some() {
                break;
            }
        }
        if let Some(shrunk) = next {
            best = shrunk;
            improved = true;
        }
    }
    if best.len() < src.len() {
        Some(best)
    } else {
        None
    }
}

// checks a shrink candidate still triggers the same hairball
fn reproduces(candidate: &str, cfg: &FuzzConfig, key: &str) -> bool {
    match run_case(candidate, cfg).0 {
        CaseOutcome::Panic { stage, message, .. } => {
            dedup_key(&stage, BugKind::Panic, &message) == key
        }
        // minimize only files panics, hangs never shrink
        _ => false,
    }
}

// stage + kind + normalized first line, the identity of a hairball
fn dedup_key(stage: &str, kind: BugKind, message: &str) -> String {
    let first = message.lines().next().unwrap_or("").trim();
    let short: String = first.chars().take(160).collect();
    format!("{stage}:{}:{}", kind.as_str(), normalize(&short))
}

// deepest effectively enabled stage, for slow cards
fn deepest_effective(stages: &crate::fuzz_config::StageConfig) -> &'static str {
    effective_stages(stages).last().copied().unwrap_or("run")
}

// collapses digit runs so `index 42` and `index 43` file as one hairball
fn normalize(signal: &str) -> String {
    let mut out = String::with_capacity(signal.len());
    let mut in_digits = false;
    for c in signal.chars() {
        if c.is_ascii_digit() {
            if !in_digits {
                out.push('#');
                in_digits = true;
            }
        } else {
            out.push(c);
            in_digits = false;
        }
    }
    out
}

// panic payload to string without ever panicking again
fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        return s.to_string();
    }
    if let Some(s) = payload.downcast_ref::<String>() {
        return s.clone();
    }
    "unknown panic payload (non-string)".to_string()
}

// gathers seed programs from configured files and dirs
fn load_corpus(cfg: &FuzzConfig) -> Vec<String> {
    let mut out = Vec::new();
    for seed in &cfg.corpus.seeds {
        collect_seed(seed, cfg, &mut out);
        if out.len() >= cfg.corpus.max_seeds {
            break;
        }
    }
    out
}

// walks one seed path, keeping small sources only
fn collect_seed(path: &Path, cfg: &FuzzConfig, out: &mut Vec<String>) {
    if out.len() >= cfg.corpus.max_seeds {
        return;
    }
    if path.is_file() {
        if let Ok(text) = std::fs::read_to_string(path)
            && text.len() <= cfg.corpus.max_seed_bytes
        {
            out.push(text);
        }
        return;
    }
    if path.is_dir() {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(path)
            .map(|r| r.filter_map(|e| e.ok()).map(|e| e.path()).collect())
            .unwrap_or_default();
        entries.sort();
        for entry in entries {
            collect_seed(&entry, cfg, out);
            if out.len() >= cfg.corpus.max_seeds {
                break;
            }
        }
    }
}

// compact terminal card, full detail lives in the file
fn print_card(card: &BugCard, out_dir: &Path) {
    println!(
        "  🐛 {} — {} (case #{}, x{})",
        card.id, card.title, card.case_no, card.occurrences
    );
    println!(
        "     card: {}",
        out_dir
            .join("cards")
            .join(format!("{}.md", card.id))
            .display()
    );
    if let Some(first) = card.message.lines().next() {
        println!("     msg: {first}");
    }
}

// writes the detailed card + reproducer files
fn save_card(card: &BugCard, cfg: &FuzzConfig) {
    let dir = cfg.run.out_dir.join("cards");
    let _ = std::fs::create_dir_all(&dir);
    let shown = card.minimized.as_deref().unwrap_or(card.input.as_str());
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let body = format!(
        "# 🐛 {id} — {title}\n\n\
        - **stage:** {stage}\n\
        - **kind:** {kind}\n\
        - **case:** #{case_no} (seed {seed}, origin {origin})\n\
        - **occurrences:** {occ}\n\
        - **case time:** {ms}ms\n\
        - **found at unix time:** {stamp}\n\
        - **reproducer:** `{id}.syx`\n\
        - **minimized:** {min}\n\n\
        ## signal\n\n```\n{message}\n```\n\n\
        ## minimized input\n\n```slynx\n{shown}\n```\n\n\
        ## original input\n\n```slynx\n{input}\n```\n\n\
        ## notes for the slynx hunt\n\n\
        - expected slynx diagnostics (`Err`) are not bugs, only panics and hangs file cards\n\
        - re-run with the same seed to replay: `lyx-meow fuzz --seed={seed} --cases={cases}`\n\
        - paste the minimized input into `examples/` to keep it as a regression test\n",
        id = card.id,
        title = card.title,
        stage = card.stage,
        kind = card.kind.as_str(),
        case_no = card.case_no,
        seed = card.seed,
        origin = card.origin,
        occ = card.occurrences,
        ms = card.ms,
        stamp = stamp,
        min = if card.minimized.is_some() {
            "yes"
        } else {
            "no"
        },
        message = card.message,
        shown = shown,
        input = card.input,
        cases = card.case_no + 1,
    );
    let _ = std::fs::write(dir.join(format!("{}.md", card.id)), body);
    if cfg.report.save_inputs {
        let _ = std::fs::write(dir.join(format!("{}.syx", card.id)), shown);
    }
}

// writes the hunt summary index
fn write_summary_file(summary: &FuzzSummary, cfg: &FuzzConfig) -> color_eyre::Result<()> {
    let mut body = format!(
        "# fuzz summary — {} cases, {} hairballs\n\n\
        - clean: {}\n- diagnostics (expected): {}\n- panics: {}\n- hangs: {}\n- slow: {}\n\
        - harness skips (generator bugs, skipped loudly): {}\n\
        - stage reach: lex {} · load {} · hir {} · ir {}\n\n## cards\n\n",
        summary.cases,
        summary.cards.len(),
        summary.clean,
        summary.diagnostics,
        summary.panics,
        summary.hangs,
        summary.slows,
        summary.harness_skips,
        summary.reached_lex,
        summary.reached_load,
        summary.reached_hir,
        summary.reached_ir,
    );
    for card in &summary.cards {
        body.push_str(&format!(
            "- **{}** — {} (case #{}, x{}) — `cards/{}.md`\n",
            card.id, card.title, card.case_no, card.occurrences, card.id
        ));
    }
    body.push_str(&format!(
        "\nreplay: `lyx-meow fuzz --seed={} --cases={}`\n",
        cfg.run.seed, cfg.run.cases
    ));
    std::fs::write(cfg.run.out_dir.join("summary.md"), body)?;
    Ok(())
}

// terminal summary after the hunt
fn print_summary(summary: &FuzzSummary) {
    println!(
        "{} hunt over: {} cases · {} clean · {} diagnostics · {} hairballs ({} panic, {} hang, {} slow)",
        theme::logo(),
        summary.cases,
        summary.clean,
        summary.diagnostics,
        summary.cards.len(),
        summary.panics,
        summary.hangs,
        summary.slows,
    );
    println!(
        "  reached: lex {} · load {} · hir {} · ir {} · harness skips {}",
        summary.reached_lex,
        summary.reached_load,
        summary.reached_hir,
        summary.reached_ir,
        summary.harness_skips,
    );
    println!("  cards: {}", summary.out_dir.join("cards").display());
    if summary.cards.is_empty() {
        println!("  no hairballs coughed up. the sunbeam is clean… for now 😺");
    }
}

#[cfg(test)]
mod tests {
    use super::{BugKind, dedup_key, effective_stages, hung_stage, panic_message};
    use crate::fuzz_config::StageConfig;

    #[test]
    fn dedup_keys_match_same_signal() {
        let a = dedup_key("hir", BugKind::Panic, "index out of bounds\nmore");
        let b = dedup_key("hir", BugKind::Panic, "index out of bounds\nother");
        assert_eq!(a, b);
    }

    #[test]
    fn dedup_keys_split_stages() {
        assert_ne!(
            dedup_key("lex", BugKind::Panic, "boom"),
            dedup_key("hir", BugKind::Panic, "boom")
        );
    }

    #[test]
    fn panic_message_reads_strings() {
        let msg = panic_message(Box::new("meow".to_string()));
        assert_eq!(msg, "meow");
    }

    #[test]
    fn dedup_folds_digit_runs() {
        let a = dedup_key("hir", BugKind::Panic, "index 42 out of bounds");
        let b = dedup_key("hir", BugKind::Panic, "index 43 out of bounds");
        assert_eq!(a, b);
    }

    #[test]
    fn hang_names_first_unreached_stage() {
        let stages = StageConfig::default();
        assert_eq!(hung_stage(&stages, &[]), "lex");
        assert_eq!(hung_stage(&stages, &["lex", "load"]), "hir");
        assert_eq!(hung_stage(&stages, &["lex", "load", "hir", "ir"]), "ir");
    }

    #[test]
    fn ir_implies_its_prerequisites() {
        let stages = StageConfig {
            lex: false,
            load: false,
            hir: false,
            ir: true,
        };
        assert_eq!(effective_stages(&stages), vec!["load", "hir", "ir"]);
    }
}
