use validated_integrity::{EntryTypes, LinkTypes, SampleEntry, UpdateSampleEntryInput};
use hdk::prelude::*;

#[hdk_extern]
fn create_sample_entry(value: String) -> ExternResult<ActionHash> {
    create_entry(EntryTypes::SampleEntry(SampleEntry { value }))
}

#[hdk_extern]
fn update_sample_entry(input: UpdateSampleEntryInput) -> ExternResult<ActionHash> {
    let update_hash = update_entry(input.original.clone(), EntryTypes::SampleEntry(SampleEntry { value: format!("update:{}", input.new_value) }))?;

    create_link(input.original, update_hash.clone(), LinkTypes::SampleLink, ())?;

    Ok(update_hash)
}
