use crate::common::Nit;

#[tarpc::service]
pub trait WayDimAPI {
    async fn get_brightness() -> Nit;
}
