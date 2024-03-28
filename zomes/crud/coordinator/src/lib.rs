use crud_integrity::{EntryTypes, SampleEntry};
use hdk::prelude::*;

#[hdk_extern]
fn create_sample_entry(value: String) -> ExternResult<ActionHash> {
    create_entry(EntryTypes::SampleEntry(SampleEntry { value }))
}

#[hdk_extern]
fn get_sample_entry(hash: ActionHash) -> ExternResult<Option<Record>> {
    get(hash, GetOptions::local())
}
