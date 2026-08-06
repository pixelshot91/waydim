use crate::{
    brightness_mapper::{BrightnessMapper, SamplePoint},
    common::{self, Nit, WayDimAPIError},
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
        let res = common::get_brightness().await;
        res.map_err(|e| WayDimAPIError::Internal(e.to_string()))
    }

    async fn set_brightness(
        self,
        context: ::tarpc::context::Context,
        nit: Nit,
    ) -> Result<(), WayDimAPIError> {
        let calibration = vec![
            SamplePoint {
                sw: 0.5,
                hw: 1,
                nits: 1.0,
            },
            SamplePoint {
                sw: 1.0,
                hw: 1,
                nits: 2.0,
            }, // Derived min hardware point
            SamplePoint {
                sw: 1.0,
                hw: 10,
                nits: 8.5,
            },
            SamplePoint {
                sw: 1.0,
                hw: 100,
                nits: 45.0,
            },
            SamplePoint {
                sw: 1.0,
                hw: 1000,
                nits: 180.0,
            },
            SamplePoint {
                sw: 1.0,
                hw: 10000,
                nits: 400.0,
            },
        ];
        let mapper = BrightnessMapper::new(calibration);

        common::set_brightness(&mapper, nit).map_err(|e| WayDimAPIError::Internal(e.to_string()))
    }
}
