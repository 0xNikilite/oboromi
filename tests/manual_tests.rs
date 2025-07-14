use oboromi::tests::run::run_tests;

#[test]
fn test_run_all_emulator_features() {
    let results = run_tests();
    for line_content in results {
        println!("{}", line_content);
    }
}
