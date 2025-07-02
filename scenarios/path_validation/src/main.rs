use holochain_wind_tunnel_runner::prelude::*;
use holochain_wind_tunnel_runner::scenario_happ_path;
use path_validated_integrity::BookEntry;

fn setup(ctx: &mut RunnerContext<HolochainRunnerContext>) -> HookResult {
    configure_app_ws_url(ctx)?;
    Ok(())
}

fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>,
) -> HookResult {
    install_app(
        ctx,
        scenario_happ_path!("path_validated"),
        &"path_validated".to_string(),
    )?;

    // There should be no books created yet.
    let books: Vec<BookEntry> = call_zome(
        ctx,
        "path_validated",
        "find_books_from_author",
        "Shakespeare",
    )?;
    assert!(books.is_empty());

    Ok(())
}

fn agent_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>,
) -> HookResult {
    // Add a book on first, and only the first, call to this behaviour.
    let () = call_zome(
        ctx,
        "path_validated",
        "add_book_entry",
        ("Shakespeare", "Romeo and Juliet"),
    )?;

    // There should only ever be a single book to this author, even on subsequent calls.
    let books: Vec<BookEntry> = call_zome(
        ctx,
        "path_validated",
        "find_books_from_author",
        "Shakespeare",
    )?;
    assert_eq!(
        books,
        [BookEntry {
            name: "Romeo and Juliet".to_string()
        }]
    );

    // Search is not case-sensitive.
    let books: Vec<BookEntry> = call_zome(
        ctx,
        "path_validated",
        "find_books_from_author",
        "shakespeare",
    )?;
    assert_eq!(
        books,
        [BookEntry {
            name: "Romeo and Juliet".to_string()
        }]
    );

    // Add another book on first, and only the first, call to this behaviour.
    let () = call_zome(
        ctx,
        "path_validated",
        "add_book_entry",
        ("Stevenson", "Strange Case of Dr Jekyll and Mr Hyde"),
    )?;

    // There should only ever be a single book to this author, even on subsequent calls.
    let books: Vec<BookEntry> =
        call_zome(ctx, "path_validated", "find_books_from_author", "Stevenson")?;
    assert_eq!(
        books,
        [BookEntry {
            name: "Strange Case of Dr Jekyll and Mr Hyde".to_string()
        }]
    );

    // Original author should still only have the original book.
    let books: Vec<BookEntry> = call_zome(
        ctx,
        "path_validated",
        "find_books_from_author",
        "Shakespeare",
    )?;
    assert_eq!(
        books,
        [BookEntry {
            name: "Romeo and Juliet".to_string()
        }]
    );

    Ok(())
}

fn main() -> WindTunnelResult<()> {
    let builder =
        ScenarioDefinitionBuilder::<HolochainRunnerContext, HolochainAgentContext>::new_with_init(
            env!("CARGO_PKG_NAME"),
        )
        .with_default_duration_s(10)
        .use_setup(setup)
        .use_agent_setup(agent_setup)
        .use_agent_behaviour(agent_behaviour)
        .use_agent_teardown(|ctx| {
            uninstall_app(ctx, None).ok();
            Ok(())
        });

    run(builder)?;

    Ok(())
}
