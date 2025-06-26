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
