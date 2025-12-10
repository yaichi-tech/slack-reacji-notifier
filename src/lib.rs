use worker::*;

mod types;
mod slack;
mod usecases;
mod handlers;

#[event(fetch)]
async fn fetch(mut req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let url = req.url()?;

    match url.path() {
        "/slack/events" => handlers::handle_slack_events(&mut req, &env).await,
        "/" => Response::ok("Slack Reacji Notifier is running!"),
        _ => Response::error("Not found", 404),
    }
}
