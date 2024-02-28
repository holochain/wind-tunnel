use std::{fmt::Debug, sync::Arc};

use crate::{
    executor::Executor,
    shutdown::{DelegatedShutdownListener, ShutdownHandle},
};

pub trait UserValuesConstraint: Default + Debug + Send + Sync + 'static {}

#[derive(Debug)]
pub struct RunnerContext<RV: UserValuesConstraint> {
    executor: Arc<Executor>,
    shutdown_handle: ShutdownHandle,
    value: RV,
}

impl<RV: UserValuesConstraint> RunnerContext<RV> {
    pub(crate) fn new(executor: Arc<Executor>, shutdown_handle: ShutdownHandle) -> Self {
        Self {
            executor,
            shutdown_handle,
            value: Default::default(),
        }
    }

    pub fn executor(&self) -> &Arc<Executor> {
        &self.executor
    }

    /// Get a new shutdown listener that will be triggered when the runner is shutdown.
    ///
    /// This is provided in case you are doing something unexpected and need to hook into the shutdown process.
    /// In general, please consider using [Executor::execute_in_place] which automatically handles shutdown.
    pub fn new_shutdown_listener(&self) -> DelegatedShutdownListener {
        self.shutdown_handle.new_listener()
    }

    pub fn get_mut(&mut self) -> &mut RV {
        &mut self.value
    }

    pub fn get(&self) -> &RV {
        &self.value
    }
}

pub struct AgentContext<RV: UserValuesConstraint, V: UserValuesConstraint> {
    runner_context: Arc<RunnerContext<RV>>,
    shutdown_listener: DelegatedShutdownListener,
    value: V,
}

impl<RV: UserValuesConstraint, V: UserValuesConstraint> AgentContext<RV, V> {
    pub(crate) fn new(
        runner_context: Arc<RunnerContext<RV>>,
        shutdown_listener: DelegatedShutdownListener,
    ) -> Self {
        Self {
            runner_context,
            shutdown_listener,
            value: Default::default(),
        }
    }

    pub fn runner_context(&self) -> &Arc<RunnerContext<RV>> {
        &self.runner_context
    }

    /// Get the shutdown listener which will be triggered when the runner is shutdown.
    ///
    /// This is provided in case you are doing something unexpected and need to hook into the shutdown process.
    /// In general, please consider using [Executor::execute_in_place] which automatically handles shutdown.
    pub fn shutdown_listener(&mut self) -> &mut DelegatedShutdownListener {
        &mut self.shutdown_listener
    }

    pub fn get_mut(&mut self) -> &mut V {
        &mut self.value
    }

    pub fn get(&self) -> &V {
        &self.value
    }
}
