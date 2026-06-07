pub mod rex {
    pub mod observability {
        pub mod v1 {
            tonic::include_proto!("rex.observability.v1");
        }
    }
    pub mod sidecar {
        pub mod v1 {
            tonic::include_proto!("rex.sidecar.v1");
        }
    }
    pub mod v1 {
        tonic::include_proto!("rex.v1");
    }
}
