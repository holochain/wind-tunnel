use hdk::prelude::*;
use remote_signal_integrity::*;

#[hdk_extern]
fn init() -> ExternResult<InitCallbackResult> {
    create_cap_grant(CapGrantEntry {
        tag: "unrestricted-access".into(),
        access: CapAccess::Unrestricted,
        functions: GrantedFunctions::Listed(HashSet::from([(
            zome_info()?.name,
            "recv_remote_signal".into(),
        )])),
    })?;

    Ok(InitCallbackResult::Pass)
}

#[hdk_extern]
fn signal_request(msg: TimedMessage) -> ExternResult<()> {
    let to = vec![msg.responder().clone()];
    send_remote_signal(ExternIO::encode(msg).map_err(|e| wasm_error!(e))?, to)
}

#[hdk_extern]
fn recv_remote_signal(signal: ExternIO) -> ExternResult<()> {
    emit_signal(AppSignal::new(signal.clone())).map_err(|e| wasm_error!("fu1: {:?}", e))?;
    let r: TimedMessage = signal
        .decode()
        .map_err(|e: SerializedBytesError| wasm_error!("fu2: {:?}", e))?;
    if let TimedMessage::TimedRequest { .. } = r {
        let r = r.to_response(sys_time()?);
        let to = vec![r.requester().clone()];
        send_remote_signal(ExternIO::encode(r).map_err(|e| wasm_error!(e))?, to)?;
    }
    Ok(())
}
