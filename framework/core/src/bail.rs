/// Return this error from an agent's behaviour function to indicate that the agent is bailing.
///
/// This should be used when an agent encounters an error that is not fatal to that agent but not
/// necessarily to the scenario. For example, if a connectivity problem occurs to a remote node then
/// the agent may bail but the scenario should continue with the other agents.
#[derive(derive_more::Error, derive_more::Display, Debug)]
pub struct AgentBailError {
    msg: String,
}

impl Default for AgentBailError {
    fn default() -> Self {
        Self {
            msg: "Agent is bailing".to_string(),
        }
    }
}
