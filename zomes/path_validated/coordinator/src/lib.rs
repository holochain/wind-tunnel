use hdk::{hdi::hash_path::path::root_hash, prelude::*};
use path_validated_integrity::{BookEntry, EntryTypes, LinkTypes};

fn recursively_create_links_from_root(
    path: TypedPath,
    book_entry_hash: EntryHash,
) -> ExternResult<()> {
    let parent_hash = if let Some(parent) = path.parent() {
        recursively_create_links_from_root(parent.clone(), book_entry_hash.clone())?;
        parent.path_entry_hash()?.into()
    } else {
        root_hash()?
    };

    if !path.exists()? {
        create_link(
            parent_hash.clone(),
            path.path_entry_hash()?,
            LinkTypes::AuthorPath,
            path.make_tag()?,
        )?;
    }
    create_link(
        parent_hash,
        book_entry_hash.clone(),
        LinkTypes::AuthorBook,
        path.make_tag()?,
    )?;

    Ok(())
}

#[hdk_extern]
fn add_book_entry(author_and_name: (String, String)) -> ExternResult<()> {
    // Use path-sharding to split author's name into single character paths.
    let path_string = format!(
        "1:{}#{}.{}",
        author_and_name.0.len(),
        author_and_name.0.to_lowercase(),
        author_and_name.1
    );
    let path = Path::from(path_string).typed(LinkTypes::AuthorPath)?;

    if path.exists()? {
        // Full path, including book name, exists so the book should exist.
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
    recursively_create_links_from_root(path.clone(), book_entry_hash.clone())?;
    create_link(
        path.path_entry_hash()?,
        book_entry_hash.clone(),
        LinkTypes::AuthorBook,
        path.make_tag()?,
    )?;

    Ok(())
}

#[hdk_extern]
fn find_books_from_author(author: String) -> ExternResult<Vec<BookEntry>> {
    let path_string = format!("1:{}#{}", author.len(), author.to_lowercase(),);
    let path = Path::from(path_string).typed(LinkTypes::AuthorPath)?;

    // Path-sharding appends an extra leaf to the path so remove it.
    let path = path
        .parent()
        .ok_or(wasm_error!("Could not get path from author"))?;

    let children_book_links = path
        .children_paths()?
        .into_iter()
        .map(|child| {
            get_links(
                GetLinksInputBuilder::try_new(
                    child.path_entry_hash()?,
                    LinkTypes::AuthorBook.try_into_filter()?,
                )?
                .build(),
            )
        })
        .collect::<ExternResult<Vec<Vec<Link>>>>()?;

    let book_entries_hashed = children_book_links
        .into_iter()
        .flatten()
        .filter_map(|link| link.target.into_entry_hash())
        .map(must_get_entry)
        .collect::<ExternResult<Vec<EntryHashed>>>()?;

    let books: Vec<_> = book_entries_hashed
        .into_iter()
        .filter_map(|entry_hashed| BookEntry::try_from(entry_hashed.content).ok())
        .collect();

    Ok(books)
}
