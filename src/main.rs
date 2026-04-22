use std::sync::mpsc;

use dotenv::dotenv;
use octocrab::Octocrab;

use crate::app::App;

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
        .items;

    let mut terminal = ratatui::init();
    let (tx, rx) = mpsc::channel();
    App::spawn_input_thread(tx.clone());

    let mut app = App::new(repos, octocrab, tx);

    let _ = app.run(&mut terminal, rx);
    ratatui::restore();

    Ok(())
}
