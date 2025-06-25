use hdk::prelude::*;
use path_validated_integrity::{EntryTypes, LinkTypes, SampleEntry};

#[hdk_extern]
fn create_sample_entry(value: String) -> ExternResult<ActionHash> {
    create_entry(EntryTypes::SampleEntry(SampleEntry { value }))
}

#[hdk_extern]
fn create_path() -> ExternResult<()> {
    Path::from("a.b").typed(LinkTypes::Path)?.ensure()
}
