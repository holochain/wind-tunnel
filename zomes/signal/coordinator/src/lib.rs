use hdk::prelude::*;

#[hdk_extern]
fn emit_10k_signals() -> ExternResult<()> {
    for i in 0..10_000 {
        emit_signal(format!("test payload {i}"))?;
    }

    Ok(())
}
