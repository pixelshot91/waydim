use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::common::{Nit, WayDimAPIError};

#[tarpc::service]
pub trait WayDimAPI {
    async fn get_brightness() -> Result<Nit, WayDimAPIError>;
    async fn set_brightness(nit: Nit) -> Result<(), WayDimAPIError>;
}
