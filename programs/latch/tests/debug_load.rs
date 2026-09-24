use litesvm::LiteSVM;

#[test]
fn debug_program_load() {
    let _ = env_logger::builder().is_test(false).filter_level(log::LevelFilter::Debug).try_init();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/latch.so"));
    let res = svm.add_program(latch::id(), bytes);
    println!("add_program result: {res:?}");
    res.unwrap();
}
