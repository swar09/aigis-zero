pub mod codec;
pub mod models;

pub mod proto {
    pub mod agent {
        tonic::include_proto!("edr.agent");
    }
    pub mod events {
        tonic::include_proto!("edr.events");
    }
    pub mod fleet {
        tonic::include_proto!("edr.fleet");
    }
}
