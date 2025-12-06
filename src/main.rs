use color_eyre::Result;
use ratatui;

mod app;
mod redis_client;

fn main() -> Result<()> {
    // color_eyre::install()?;

    let redis_client = redis_client::RedisClient::new().unwrap();
    let mut terminal = ratatui::init();
    let app_result = app::App::new(redis_client).unwrap().run(terminal);
    ratatui::restore();
    app_result
}
