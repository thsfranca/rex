fn main() {
    println!("cargo:rerun-if-changed=../../proto/rex/v1/rex.proto");

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&["../../proto/rex/v1/rex.proto"], &["../../proto"])
        .expect("failed to compile rex.v1 protobufs");
}
