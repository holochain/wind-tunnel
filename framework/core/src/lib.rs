mod shutdown;

pub mod prelude {
    pub use crate::shutdown::{DelegatedShutdownListener, ShutdownHandle, ShutdownSignalError};
}
