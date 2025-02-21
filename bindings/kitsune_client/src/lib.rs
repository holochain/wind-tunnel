use anyhow::Context;
use bytes::Bytes;
use kitsune2::default_builder;
use kitsune2_api::{
    BoxFut, Builder, DhtArc, DynKitsune, DynLocalAgent, DynSpace, DynSpaceHandler, Id, K2Result,
    KitsuneHandler, LocalAgent, OpId, SpaceHandler, SpaceId, StoredOp, Timestamp,
};
use kitsune2_core::{
    factories::{
        config::{CoreBootstrapConfig, CoreBootstrapModConfig},
        MemoryOp,
    },
    Ed25519LocalAgent,
};
use kitsune2_gossip::{K2GossipConfig, K2GossipModConfig};
use kitsune2_transport_tx5::config::{Tx5TransportConfig, Tx5TransportModConfig};
use op_store::WtOpStoreFactory;
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;
use wind_tunnel_instruments::prelude::{ReportMetric, Reporter};
use wind_tunnel_instruments_derive::wind_tunnel_instrument;

mod op_store;

#[derive(Debug)]
struct WtSpaceHandler;
impl SpaceHandler for WtSpaceHandler {}

#[derive(Debug)]
struct WtKitsuneHandler;
impl KitsuneHandler for WtKitsuneHandler {
    fn create_space(&self, _space: SpaceId) -> BoxFut<'_, K2Result<DynSpaceHandler>> {
        let space_handler: DynSpaceHandler = Arc::new(WtSpaceHandler);
        Box::pin(async move { Ok(space_handler) })
    }
}

#[derive(Debug)]
struct State {
    space: DynSpace,
    _kitsune: DynKitsune,
    agent: DynLocalAgent,
}

#[derive(Debug)]
pub struct WtChatter {
    state: Arc<Mutex<State>>,
    reporter: Arc<Reporter>,
}

impl WtChatter {
    pub async fn create(
        bootstrap_server_url: &str,
        signal_server_url: &str,
        space_id: &str,
        reporter: Arc<Reporter>,
    ) -> anyhow::Result<Self> {
        let agent = Arc::new(Ed25519LocalAgent::default());
        agent.set_tgt_storage_arc_hint(DhtArc::FULL);
        let kitsune_builder = Builder {
            op_store: Arc::new(WtOpStoreFactory::new(
                agent.agent().clone(),
                reporter.clone(),
            )),
            ..default_builder()
        }
        .with_default_config()?;
        kitsune_builder
            .config
            .set_module_config(&CoreBootstrapModConfig {
                core_bootstrap: CoreBootstrapConfig {
                    server_url: bootstrap_server_url.to_string(),
                    ..Default::default()
                },
            })?;
        kitsune_builder
            .config
            .set_module_config(&Tx5TransportModConfig {
                tx5_transport: Tx5TransportConfig {
                    server_url: signal_server_url.to_string(),
                    signal_allow_plain_text: true,
                    ..Default::default()
                },
            })?;
        kitsune_builder
            .config
            .set_module_config(&K2GossipModConfig {
                k2_gossip: K2GossipConfig {
                    initiate_interval_ms: 1000,
                    min_initiate_interval_ms: 900,
                    ..Default::default()
                },
            })?;
        let kitsune = kitsune_builder.build().await?;
        kitsune.register_handler(Arc::new(WtKitsuneHandler)).await?;
        let space = kitsune
            .space(SpaceId(Id(Bytes::copy_from_slice(space_id.as_bytes()))))
            .await?;

        log::info!("created chatter with id {}", agent.agent());

        Ok(Self {
            state: Arc::new(Mutex::new(State {
                _kitsune: kitsune,
                space,
                agent,
            })),
            reporter,
        })
    }

    #[wind_tunnel_instrument]
    pub async fn join_space(&self) -> anyhow::Result<()> {
        let state_lock = self.state.lock().await;
        state_lock
            .space
            .local_agent_join(state_lock.agent.clone())
            .await?;

        // Wait for agent to publish their info to the bootstrap & peer store.
        tokio::time::timeout(Duration::from_secs(20), async {
            loop {
                tokio::time::sleep(Duration::from_millis(1000)).await;
                let maybe_agent = match state_lock
                    .space
                    .peer_store()
                    .get(state_lock.agent.agent().clone())
                    .await
                {
                    Ok(maybe_agent) => maybe_agent,
                    Err(err) => {
                        log::error!("failure to query peer store: {err}");
                        continue;
                    }
                };
                if maybe_agent.is_some() {
                    break;
                }
            }
        })
        .await
        .context("failure to join space")
    }

    #[wind_tunnel_instrument]
    pub async fn say(&self, message: &str) -> anyhow::Result<Vec<OpId>> {
        let state = self.state.lock().await;
        let message_op = MemoryOp::new(Timestamp::now(), message.into());
        let message_ids = state
            .space
            .op_store()
            .process_incoming_ops(vec![message_op.into()])
            .await
            .context("failure to write ops to the store")?;
        state
            .space
            .inform_ops_stored(
                message_ids
                    .clone()
                    .into_iter()
                    .map(|message_id| StoredOp {
                        created_at: Timestamp::now(),
                        op_id: message_id,
                    })
                    .collect(),
            )
            .await?;
        log::info!("agent {} said {}", state.agent.agent(), message);

        self.reporter.add_custom(
            ReportMetric::new("message said")
                .with_tag("agent_id", state.agent.agent().to_string())
                .with_field("message_said", message_ids.len() as u32),
        );

        Ok(message_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kitsune2_bootstrap_srv::{BootstrapSrv, Config};
    use std::time::Duration;
    use wind_tunnel_core::prelude::ShutdownHandle;
    use wind_tunnel_instruments::{ReportConfig, Reporter};

    pub(crate) fn test_reporter() -> Arc<Reporter> {
        let runtime = tokio::runtime::Handle::current();
        let shutdown_listener = ShutdownHandle::new().new_listener();
        Arc::new(
            ReportConfig::new("".to_string(), "".to_string())
                .enable_in_memory()
                .init_reporter(&runtime, shutdown_listener)
                .unwrap(),
        )
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn say_something_to_other_chatter() {
        env_logger::init();
        let bootstrap_server =
            tokio::task::spawn_blocking(|| BootstrapSrv::new(Config::testing()).unwrap())
                .await
                .unwrap();
        let bootstrap_server_url = format!("http://{}", bootstrap_server.listen_addrs()[0]);
        let signal_server_url = format!("ws://{}", bootstrap_server.listen_addrs()[0]);

        let reporter = test_reporter();
        let space_id = Timestamp::now().as_micros().to_string();
        let chatter_1 = WtChatter::create(
            &bootstrap_server_url,
            &signal_server_url,
            &space_id,
            reporter.clone(),
        )
        .await
        .unwrap();
        let chatter_2 = WtChatter::create(
            &bootstrap_server_url,
            &signal_server_url,
            &space_id,
            reporter,
        )
        .await
        .unwrap();
        let agent_1 = chatter_1.state.lock().await.agent.agent().clone();
        let agent_2 = chatter_2.state.lock().await.agent.agent().clone();
        chatter_1.join_space().await.unwrap();
        chatter_2.join_space().await.unwrap();

        // Bootstrapping takes about 5 seconds.
        tokio::time::sleep(Duration::from_secs(5)).await;

        // Each chatter says 3 messages.
        let mut all_message_ids_1 = vec![];
        let mut all_message_ids_2 = vec![];
        for i in 0..3 {
            let message_1 = format!("hello there {} {}", agent_1, i);
            let message_2 = format!("hello there {} {}", agent_2, i);
            let mut message_ids_1 = chatter_1.say(&message_1).await.unwrap();
            let mut message_ids_2 = chatter_2.say(&message_2).await.unwrap();
            all_message_ids_1.append(&mut message_ids_1);
            all_message_ids_2.append(&mut message_ids_2);
            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        // Wait for both chatters to have all messages in store.
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                tokio::time::sleep(Duration::from_millis(100)).await;
                let ops_1 = chatter_2
                    .state
                    .lock()
                    .await
                    .space
                    .op_store()
                    .retrieve_ops(all_message_ids_1.clone())
                    .await
                    .unwrap();
                let ops_2 = chatter_1
                    .state
                    .lock()
                    .await
                    .space
                    .op_store()
                    .retrieve_ops(all_message_ids_2.clone())
                    .await
                    .unwrap();
                if ops_1.len() == all_message_ids_1.len() && ops_2.len() == all_message_ids_2.len()
                {
                    break;
                } else {
                    println!("ops 1 len {}/{}", ops_1.len(), all_message_ids_1.len());
                    println!("ops 2 len {}/{}", ops_2.len(), all_message_ids_2.len());
                }
            }
        })
        .await
        .unwrap();
    }
}
