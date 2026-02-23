use std::ops::Deref;

use kube::Client;

use crate::state::State;

pub struct Context {
    client: Client,
    state: State,
}

impl From<(Client, State)> for Context {
    fn from((client, state): (Client, State)) -> Self {
        Self { client, state }
    }
}

impl Deref for Context {
    type Target = State;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl Context {
    pub fn client(&self) -> Client {
        self.client.clone()
    }
}
