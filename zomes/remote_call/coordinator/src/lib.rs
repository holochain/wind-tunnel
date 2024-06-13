use hdk::prelude::*;
use remote_call_integrity::*;

#[hdk_extern]
fn init() -> ExternResult<InitCallbackResult> {
    create_cap_grant(CapGrantEntry {
        tag: "unrestricted-access".into(),
        access: CapAccess::Unrestricted,
        functions: GrantedFunctions::Listed(BTreeSet::from([(
            zome_info()?.name,
            "echo_timestamp".into(),
        )])),
    })?;

    Ok(InitCallbackResult::Pass)
}

#[hdk_extern]
fn call_echo_timestamp(to: AgentPubKey) -> ExternResult<TimedResponse> {
    let response = call_remote(
        to,
        zome_info()?.name,
        "echo_timestamp".into(),
        None,
        &TimedRequest { value: sys_time()? },
    )?;

    match response {
        ZomeCallResponse::Ok(extern_io) => Ok(extern_io.decode().map_err(|e| wasm_error!(e))?),
        e => Err(wasm_error!(WasmErrorInner::Guest(format!("{:?}", e)))),
    }
}

#[hdk_extern]
fn echo_timestamp(request: TimedRequest) -> ExternResult<TimedResponse> {
    Ok(TimedResponse {
        request_value: request.value,
        value: sys_time()?,
    })
}
