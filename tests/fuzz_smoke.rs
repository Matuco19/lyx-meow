// SPDX-License-Identifier: GPL-3.0-only
use lyx_meow::fuzz_config::FuzzConfig;

#[test]
fn fuzz_runs_headless_smoke() {
    let mut cfg = FuzzConfig::default();
    cfg.run.cases = 8;
    cfg.run.seed = 42;
    cfg.run.timeout_ms = 5000;
    cfg.run.minimize = false;
    cfg.run.out_dir = std::env::temp_dir().join(format!(
        "lyx-fuzz-smoke-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock should run")
            .as_nanos()
    ));
    let summary = lyx_meow::fuzz::run_with_config(&cfg).expect("smoke hunt should finish");
    assert_eq!(summary.cases, 8);
    assert!(summary.out_dir.join("summary.md").exists());
    let _ = std::fs::remove_dir_all(&summary.out_dir);
}
