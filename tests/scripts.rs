//! Every script in `scripts/` must parse and run without panicking.

use platosku2::script::Script;
use platosku2::sim::Sim;

#[test]
fn all_scripts_parse_and_run() {
    let mut paths: Vec<_> = std::fs::read_dir("scripts")
        .expect("scripts/ folder")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "txt"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no scripts found");

    for path in paths {
        let path = path.to_string_lossy().to_string();
        let script = Script::load(&path).unwrap_or_else(|e| panic!("{e}"));
        let game = script.build_game().unwrap_or_else(|e| panic!("{e}"));
        let mut sim = Sim::new(game);
        let mut snaps = 0;
        sim.run_script(&script, |_, _| snaps += 1);
        assert_eq!(sim.tick(), script.total_ticks(), "{path}");
        assert!(snaps > 0, "{path}: has no snap markers");
    }
}
