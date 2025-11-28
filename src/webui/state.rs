use crate::{configuration::ConfigurationManager, server_runner::ServerRunnerHandle};

#[derive(Debug, Clone)]
pub struct WebState {
    pub config: ConfigurationManager,
    pub runner_handle: ServerRunnerHandle,
}
