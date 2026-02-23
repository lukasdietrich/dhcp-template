use dhcp_template_api::{Refresh, Scope, Update, controller_service_server, update::Data};
use tonic::{Request, Response, Status};
use tracing::{Level, instrument};

use crate::state::{self, State};

pub struct ControllerService {
    state: State,
}

impl From<State> for ControllerService {
    fn from(state: State) -> Self {
        Self { state }
    }
}

impl ControllerService {
    #[instrument(
        skip_all,
        fields(token = update.token),
        ret(level = Level::DEBUG),
        err(level = Level::WARN),
    )]
    async fn push_node(&self, update: Update) -> Result<Refresh, Status> {
        let status = match &update.data {
            Some(Data::Full(node)) => self.state.insert((node, update.token)).await,
            Some(Data::Shallow(shallow)) => self.state.status((shallow, update.token)).await,
            None => state::Status::Unknown,
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

        Ok(refresh)
    }
}

#[async_trait::async_trait]
impl controller_service_server::ControllerService for ControllerService {
    async fn push_node(&self, request: Request<Update>) -> Result<Response<Refresh>, Status> {
        let update = request.into_inner();
        let refresh = self.push_node(update).await?;

        Ok(refresh.into())
    }
}
