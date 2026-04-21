use std::{sync::mpsc, thread};

use dotenv::dotenv;
use octocrab::Octocrab;

use crate::app::{App, Event};

mod app;

#[tokio::main]
async fn main() -> octocrab::Result<()> {
    dotenv().ok();
    let token = std::env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN not set");

    let octocrab = Octocrab::builder().personal_token(token).build()?;

    let repos: Vec<octocrab::models::Repository> = octocrab
        .current()
        .list_repos_for_authenticated_user()
        .type_("all")
        .sort("updated")
        .per_page(100)
        .send()
        .await?
        .into_iter()
        .collect();

    let mut terminal = ratatui::init();

    let (event_tx, event_rx) = mpsc::channel::<Event>();

    let mut app = App::new(repos);

    let tx_to_input_events = event_tx.clone();
    // Fix this
    thread::spawn(move || {});

    let app_result = app.run(&mut terminal, event_rx);

    ratatui::restore();

    Ok(())
}
