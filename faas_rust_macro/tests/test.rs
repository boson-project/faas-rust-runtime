#[test]
fn test() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/1-input-1-output.rs");
    //t.pass("tests/ui/1-optional-input-1-output.rs");
    //t.pass("tests/ui/1-optional-input-1-optional-output.rs");
    //t.pass("tests/ui/1-input-1-optional-output.rs");
    //t.pass("tests/ui/1-optional-output.rs");
    //t.pass("tests/ui/1-vec-input-1-optional-output.rs");
    //t.pass("tests/ui/1-vec-output.rs");
    //t.pass("tests/ui/2-input-1-output.rs");
    //t.pass("tests/ui/2-input-with-optional-1-output.rs");
    //t.pass("tests/ui/1-map-output.rs");
    //t.compile_fail("tests/ui/invalid-2-input-with-map-1-output.rs");
    //t.compile_fail("tests/ui/invalid-2-input-with-vec-1-output.rs");
}
