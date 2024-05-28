use hdk::prelude::*;

#[hdk_extern]
fn init() -> ExternResult<InitCallbackResult> {
    Ok(InitCallbackResult::Pass)
}
