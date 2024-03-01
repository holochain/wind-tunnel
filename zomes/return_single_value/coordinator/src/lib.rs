use hdk::prelude::*;

#[hdk_extern]
fn get_value(_: ()) -> ExternResult<u32> {
    Ok(5)
}
