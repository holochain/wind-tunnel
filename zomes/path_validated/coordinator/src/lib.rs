use hdk::{hdi::hash_path::path::root_hash, prelude::*};
use path_validated_integrity::{BookEntry, EntryTypes, LinkTypes};

fn recursively_create_links_from_root(path: TypedPath) -> ExternResult<()> {
    if path.exists()? {
        return Ok(()); // Path already exists, therefore, so must its parents.
    }

    if let Some(parent) = path.parent() {
        recursively_create_links_from_root(parent.clone())?;
        create_link(
            parent.path_entry_hash()?,
            path.path_entry_hash()?,
            LinkTypes::AuthorPath,
            path.make_tag()?,
        )?;
    } else {
        create_link(
            root_hash()?,
            path.path_entry_hash()?,
            LinkTypes::AuthorPath,
            path.make_tag()?,
        )?;
    }

    Ok(())
}

#[hdk_extern]
fn add_book_entry(author_and_name: (String, String)) -> ExternResult<()> {
    let path = Path::from(author_and_name.0.to_lowercase()).typed(LinkTypes::AuthorPath)?;

    if path.exists()? {
        return Ok(());
    }

    let book_action_hash = create_entry(EntryTypes::BookEntry(BookEntry {
        name: author_and_name.1,
    }))?;
    let book_action = must_get_action(book_action_hash)?;
    let book_entry_hash = book_action
        .action()
        .entry_hash()
        .expect("create book action has not entry hash");
    recursively_create_links_from_root(path.clone())?;
    create_link(
        path.path_entry_hash()?,
        book_entry_hash.clone(),
        LinkTypes::AuthorPath,
        "book",
    )?;

    Ok(())
}

#[hdk_extern]
fn find_books_from_author(author: String) -> ExternResult<Vec<BookEntry>> {
    let path = Path::from(author.to_lowercase()).typed(LinkTypes::AuthorPath)?;

    let book_entries_hashed = get_links(
        GetLinksInputBuilder::try_new(
            path.path_entry_hash()?,
            LinkTypes::AuthorPath.try_into_filter()?,
        )?
        .tag_prefix("book".into())
        .build(),
    )?
    .into_iter()
    .filter_map(|link| link.target.into_entry_hash())
    .map(must_get_entry)
    .collect::<ExternResult<Vec<EntryHashed>>>()?;

    let books = book_entries_hashed
        .into_iter()
        .filter_map(|entry_hashed| BookEntry::try_from(entry_hashed.content).ok())
        .collect();

    Ok(books)
}
