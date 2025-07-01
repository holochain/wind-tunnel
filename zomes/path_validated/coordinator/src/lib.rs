use hdk::prelude::*;
use path_validated_integrity::{BookEntry, EntryTypes, LinkTypes};

#[hdk_extern]
fn add_book_entry(name: String) -> ExternResult<ActionHash> {
    create_entry(EntryTypes::BookEntry(BookEntry { name }))
}

#[hdk_extern]
fn create_path() -> ExternResult<()> {
    Path::from("a.b").typed(LinkTypes::Path)?.ensure()
}
