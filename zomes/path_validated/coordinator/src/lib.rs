use hdk::prelude::*;
use path_validated_integrity::{EntryTypes, LinkTypes, SampleEntry};

#[hdk_extern]
fn create_sample_entry() -> ExternResult<ActionHash> {
    create_entry(EntryTypes::SampleEntry(SampleEntry { updated: false }))
}

#[hdk_extern]
fn create_path() -> ExternResult<()> {
    Path::from("a.b").typed(LinkTypes::Path)?.ensure()
}

#[hdk_extern]
fn update_sample_entry(original_entry: ActionHash) -> ExternResult<ActionHash> {
    let update_hash = update_entry(
        original_entry.clone(),
        EntryTypes::SampleEntry(SampleEntry { updated: true }),
    )?;

    create_link(
        original_entry,
        update_hash.clone(),
        LinkTypes::SampleLink,
        (),
    )?;

    Ok(update_hash)
}
