use std::{fmt::Debug, sync::Arc};

use crate::{executor::Executor, shutdown::DelegatedShutdownListener};

pub trait UserValuesConstraint: Default + Debug + Send + Sync + 'static {}

#[derive(Debug)]
pub struct RunnerContext<RV: UserValuesConstraint> {
    executor: Arc<Executor>,
    value: RV,
}

impl<RV: UserValuesConstraint> RunnerContext<RV> {
    pub fn new(executor: Executor) -> Self {
        Self {
            executor: Arc::new(executor),
            value: Default::default(),
        }
    }

    pub fn executor(&self) -> &Arc<Executor> {
        &self.executor
    }

    pub fn get_mut(&mut self) -> &mut RV {
        &mut self.value
    }

    pub fn get(&self) -> &RV {
        &self.value
    }
}

pub struct Context<RV: UserValuesConstraint, V: UserValuesConstraint> {
    runner_context: Arc<RunnerContext<RV>>,
    shutdown_listener: DelegatedShutdownListener,
    value: V,
}

impl<RV: UserValuesConstraint, V: UserValuesConstraint> Context<RV, V> {
    pub(crate) fn new(runner_context: Arc<RunnerContext<RV>>, shutdown_listener: DelegatedShutdownListener) -> Self {
        Self {
            runner_context,
            shutdown_listener,
            value: Default::default(),
        }
    }

    pub fn runner_context(&self) -> &Arc<RunnerContext<RV>> {
        &self.runner_context
    }

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
