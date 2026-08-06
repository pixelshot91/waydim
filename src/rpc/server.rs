use crate::{common::Nit, rpc::waydim_api::WayDimAPI};

#[derive(Clone)]
pub struct WayDimServer {}

impl WayDimAPI for WayDimServer {
    async fn get_brightness(self, context: ::tarpc::context::Context) -> Nit {
        todo!()
    }
}
