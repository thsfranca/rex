fn main() {
    println!("cargo:rerun-if-changed=../../proto/rex/v1/rex.proto");
    println!("cargo:rerun-if-changed=../../proto/rex/sidecar/v1/sidecar.proto");

    let protos = [
        "../../proto/rex/v1/rex.proto",
        "../../proto/rex/sidecar/v1/sidecar.proto",
    ];
    let includes = ["../../proto"];

    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&protos, &includes)
        .expect("failed to compile rex protobufs");
}
