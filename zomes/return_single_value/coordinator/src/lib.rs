use hdk::prelude::*;

#[hdk_extern]
fn get_value() -> ExternResult<u32> {
    Ok(5)
}
