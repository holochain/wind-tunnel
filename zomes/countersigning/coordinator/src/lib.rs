use countersigning_integrity::*;
use hdk::prelude::*;

#[repr(u8)]
pub enum Roles {
    Vendor = 1,
    Taster = 2,
}

impl From<Roles> for Role {
    fn from(value: Roles) -> Self {
        Role(value as u8)
    }
}

#[hdk_extern]
fn init() -> ExternResult<InitCallbackResult> {
    let mut fns = HashSet::new();
    fns.insert((zome_info()?.name, "recv_remote_signal".into()));
    fns.insert((zome_info()?.name, "call_remote_signal".into()));
    let functions = GrantedFunctions::Listed(fns);
    create_cap_grant(CapGrantEntry {
        tag: "".into(),
        // empty access converts to unrestricted
        access: ().into(),
        functions,
    })?;

    Ok(InitCallbackResult::Pass)
}

#[hdk_extern]
fn recv_remote_signal(signal: Signals) -> ExternResult<()> {
    emit_signal(signal)
}

/// Send a remote signal to a specific agent by calling this zome function remotely.
#[hdk_extern]
fn call_remote_signal(signal: Signals) -> ExternResult<()> {
    emit_signal(signal)
}

#[hdk_extern]
fn initiator_hello() -> ExternResult<String> {
    Ok("Hello!".to_string())
}

#[hdk_extern]
fn participant_hello() -> ExternResult<()> {
    create_link(
        hash_entry(EntryTypes::ParticipantBase(ParticipantBase))?,
        agent_info()?.agent_initial_pubkey,
        LinkTypes::Participant,
        (),
    )?;

    Ok(())
}

#[hdk_extern]
fn list_participants() -> ExternResult<Vec<AgentPubKey>> {
    let links = get_links(
        LinkQuery::try_new(
            hash_entry(EntryTypes::ParticipantBase(ParticipantBase))?,
            LinkTypes::Participant,
        )?,
        GetStrategy::default(),
    )?;
    Ok(links
        .into_iter()
        .map(|link| link.target.clone().try_into().unwrap())
        .collect())
}

#[hdk_extern]
fn start_two_party(with_other: AgentPubKey) -> ExternResult<PreflightResponse> {
    let my_agent_info = agent_info()?;

    let entry = ImportantAgreement {
        best_ice_cream_flavour: "strawberry".to_string(),
    };

    let entry_hash = hash_entry(EntryTypes::ImportantAgreement(entry.clone()))?;

    let session_times = session_times_from_millis(5_000)?;
    let request = PreflightRequest::try_new(
        entry_hash,
        vec![
            (
                my_agent_info.agent_initial_pubkey,
                vec![Roles::Vendor.into()],
            ),
            (with_other.clone(), vec![Roles::Taster.into()]),
        ],
        Vec::with_capacity(0),
        0,
        false,
        session_times,
        ActionBase::Create(CreateBase::new(
            UnitEntryTypes::ImportantAgreement.try_into()?,
        )),
        PreflightBytes(entry.best_ice_cream_flavour.into_bytes()),
    )
    .map_err(|e| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "Failed to create countersigning request: {e:?}"
        )))
    })?;

    // Accept ours now and then Holochain should wait for the other party to join the session
    let my_acceptance = accept_countersigning_preflight_request(request.clone())?;

    let response = match &my_acceptance {
        PreflightRequestAcceptance::Accepted(response) => response.clone(),
        e => {
            return Err(wasm_error!(WasmErrorInner::Guest(format!(
                "Unexpected response: {e:?}"
            ))));
        }
    };

    // Let the other party know about the request
    let result = call_remote(
        with_other,
        zome_info()?.name,
        "call_remote_signal".into(),
        None,
        Signals::AcceptedRequest(AcceptedRequest {
            preflight_request: request.clone(),
            preflight_response: response.clone(),
        }),
    )?;

    if let ZomeCallResponse::Ok(_) = result {
        Ok(response)
    } else {
        Err(wasm_error!(WasmErrorInner::Guest(
            "Failed to send remote signal".to_string()
        )))
    }
}

#[hdk_extern]
fn accept_two_party(request: PreflightRequest) -> ExternResult<PreflightResponse> {
    let initiating_agent = request.signing_agents.first().unwrap().0.clone();

    // Pre-flight check
    let flavour = String::from_utf8_lossy(&request.preflight_bytes.0);
    if flavour != "strawberry" {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Only chocolate is accepted".to_string()
        )));
    }

    // Accept the request and send the acceptance back to the requester
    let my_accept = accept_countersigning_preflight_request(request)?;
    match my_accept {
        PreflightRequestAcceptance::Accepted(response) => {
            let result = call_remote(
                initiating_agent,
                zome_info()?.name,
                "call_remote_signal".into(),
                None,
                Signals::Response(response.clone()),
            )?;

            if let ZomeCallResponse::Ok(_) = result {
                Ok(response)
            } else {
                Err(wasm_error!(WasmErrorInner::Guest(
                    "Failed to send remote signal".to_string()
                )))
            }
        }
        e => Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Unexpected response: {e:?}"
        )))),
    }
}

#[hdk_extern]
fn commit_two_party(responses: Vec<PreflightResponse>) -> ExternResult<()> {
    let inner = ImportantAgreement {
        best_ice_cream_flavour: "strawberry".to_string(),
    };

    // TODO This doesn't belong here, we're asking for agent activity creates when Holochain should
    //      know that we need this and send it to us...
    let my_key = agent_info()?.agent_initial_pubkey;
    let signing_agents = &responses.first().unwrap().request.signing_agents;
    for (agent, _) in signing_agents {
        // No need to check our own activity, only the remote.
        if agent == &my_key {
            continue;
        }

        get_agent_activity(
            agent.clone(),
            ChainQueryFilter::new(),
            ActivityRequest::Full,
            GetOptions::default(),
        )?;
    }

    let entry = Entry::CounterSign(
        Box::new(
            CounterSigningSessionData::try_from_responses(responses, vec![]).map_err(
                |countersigning_error| {
                    wasm_error!(WasmErrorInner::Guest(countersigning_error.to_string()))
                },
            )?,
        ),
        inner.clone().try_into()?,
    );

    let agreement = EntryTypes::ImportantAgreement(inner);
    let entry_def_index = ScopedEntryDefIndex::try_from(&agreement)?;
    let visibility = EntryVisibility::from(&agreement);

    create(CreateInput::new(
        entry_def_index,
        visibility,
        entry,
        // Countersigned entries MUST have strict ordering.
        ChainTopOrdering::Strict,
    ))?;

    Ok(())
}
