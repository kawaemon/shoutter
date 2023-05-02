fn main() {
    prost_build::Config::new()
        .out_dir("./src")
        .compile_protos(&["./diff_confused.proto"], &[] as &[&str])
        .unwrap();
}
