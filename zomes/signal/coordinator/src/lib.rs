use hdk::prelude::*;

#[hdk_extern]
fn emit_10k_signals() -> ExternResult<()> {
    for _ in 0..10_000 {
        emit_signal("test payload")?;
    }

    Ok(())
}
