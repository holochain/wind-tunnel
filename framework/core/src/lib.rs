mod bail;
mod shutdown;

pub mod prelude {
    pub use crate::bail::AgentBailError;
    pub use crate::shutdown::{DelegatedShutdownListener, ShutdownHandle, ShutdownSignalError};
}
