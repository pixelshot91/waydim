use crate::{
    common::{Nit, WayDimAPIError},
    driver,
    rpc::waydim_api::WayDimAPI,
};

#[derive(Clone)]
pub struct WayDimServer {}

impl WayDimAPI for WayDimServer {
    async fn get_brightness(
        self,
        context: ::tarpc::context::Context,
    ) -> Result<Nit, WayDimAPIError> {
        let res = driver::get_brightness().await;
        res.map_err(|e| WayDimAPIError::Internal(e.to_string()))
    }
}
