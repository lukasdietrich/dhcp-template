use dhcp_template_api::{Refresh, Scope, Update, controller_service_server, update::Data};
use log::{error, trace};
use tonic::{Request, Response, Status};

use crate::state::{self, State};

pub struct ControllerService {
    state: State,
}

impl ControllerService {
    pub fn new(state: State) -> Self {
        Self { state }
    }
}

#[async_trait::async_trait]
impl controller_service_server::ControllerService for ControllerService {
    async fn push_node(&self, request: Request<Update>) -> Result<Response<Refresh>, Status> {
        let update = request.into_inner();
        trace!("Received {:#?}.", update);

        let status = match &update.data {
            Some(Data::Full(node)) => self.state.insert((node, update.token)).await,
            Some(Data::Shallow(shallow)) => self.state.status((shallow, update.token)).await,
            None => {
                error!("Received empty update!");
                state::Status::Unknown
            }
        };

        let refresh = match status {
            state::Status::Deprecated | state::Status::Unknown => Refresh {
                backoff_seconds: Default::default(),
                scope: Scope::Full.into(),
            },
            state::Status::Ok(backoff_seconds) => Refresh {
                backoff_seconds,
                scope: Scope::Shallow.into(),
            },
        };

        Ok(refresh.into())
    }
}
