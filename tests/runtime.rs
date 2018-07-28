extern crate nvptx;

#[test]
fn compiler_rt() {
    let rt = nvptx::get_compiler_rt().unwrap();
    println!("Compiler runtimes = {:?}", rt);
    assert_eq!(rt.len(), 1);
}
