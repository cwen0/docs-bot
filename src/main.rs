#![allow(unused)]
use anyhow::Context as _;
use std::{env, thread};
use std::net::SocketAddr;
use std::option::Option::Some;
use std::sync::{Arc, mpsc};
use futures::future::FutureExt;
use futures::StreamExt;
use reqwest::Client;
use uuid::Uuid;
use docsbot::{logger, db, webhook, github};
use docsbot::github::PullRequestEvent;
use docsbot::handlers::{Context, handle_pr_task};
use hyper::{header, Body, Request, Response, Server, StatusCode, Method};

async fn serve_req(
    req: Request<Body>,
    ctx: Arc<Context>,
    sender: mpsc::Sender<PullRequestEvent>,
) -> Result<Response<Body>, hyper::Error> {
    log::info!("request = {:?}", req);
    let (req, body_stream) = req.into_parts();

    match (req.method, req.uri.path()) {
        (Method::POST, "/github-hook") => {
            let event = if let Some(ev) = req.headers.get("X-GitHub-Event") {
                let ev = match ev.to_str().ok() {
                    Some(v) => v,
                    None => {
                        return Ok(Response::builder()
                            .status(StatusCode::BAD_REQUEST)
                            .body(Body::from("X-GitHub-Event header must be UTF-8 encoded"))
                            .unwrap());
                    }
                };

                match ev.parse::<webhook::EventName>() {
                    Ok(v) => v,
                    Err(_) => unreachable!(),
                }
            } else {
                return Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::from("X-GitHub-Event header must be set"))
                    .unwrap());
            };

            log::debug!("event={}", event);

            let mut c = body_stream;
            let mut payload = Vec::new();
            while let Some(chunk) = c.next().await {
                let chunk = chunk?;
                payload.extend_from_slice(&chunk);
            }

            let payload = match String::from_utf8(payload) {
                Ok(p) => p,
                Err(_) => {
                    return Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Body::from("Payload must be UTF-8"))
                        .unwrap());
                }
            };

            // TODO: check signature

            match webhook::webhook(event, payload, &ctx, sender).await {
                Ok(true) => Ok(Response::new(Body::from("processed request"))),
                Ok(false) => Ok(Response::new(Body::from("ignored request"))),
                Err(err) => {
                    log::error!("request failed: {:?}", err);
                    Ok(Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::from(format!("request failed: {:?}", err)))
                        .unwrap())
                }
            }
        },
        _ => {
             Ok(Response::builder()
                 .status(StatusCode::NOT_FOUND)
                 .header(header::ALLOW, "POST")
                 .body(Body::empty())
                 .unwrap())
        }
    }
}

async fn run_server(
    ctx: Arc<Context>,
    addr: SocketAddr,
    sender: mpsc::Sender<PullRequestEvent>,
) -> anyhow::Result<()> {
    log::info!("Listening on http://{}", addr);
    let svc = hyper::service::make_service_fn(move |_conn| {
        let ctx = ctx.clone();
        let sender = sender.clone();
        async move {
            let uuid = Uuid::new_v4();
            Ok::<_, hyper::Error>(hyper::service::service_fn(move |req| {
                logger::LogFuture::new(
                    uuid,
                    serve_req(req, ctx.clone(), sender.clone()).map(move |mut resp| {
                        if let Ok(resp) = &mut resp {
                            resp.headers_mut()
                                .insert("X-Request-Id", uuid.to_string().parse().unwrap());
                        }
                        log::info!("response = {:?}", resp);
                        resp
                    }),
                )
            }))
        }
    });

    let serve_future = Server::bind(&addr).serve(svc);

    serve_future.await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    logger::init();

    let port = env::var("PORT")
        .ok()
        .map(|p| p.parse::<u16>().expect("parsed PORT"))
        .unwrap_or(8000);

    let (tx, rx): (mpsc::Sender<PullRequestEvent>, mpsc::Receiver<PullRequestEvent>) = mpsc::channel();

    let client = Client::new();
    let gh = github::GithubClient::new_with_default_token(client.clone());

    let ctx = Arc::new(Context{
        github: gh,
        // db_conn: conn.unwrap(),
        username: String::from("docsbot"),
        // pr_task_sender: tx,
        // pr_task_receiver: rx,
    });

    let addr:SocketAddr = ([0, 0, 0, 0], port).into();

    // log::info!("server addr: {}", addr);
    let ctx_ = ctx.clone();
    tokio::spawn(async move {
        if let Err(e) = run_server(ctx_, addr, tx).await{
            eprintln!("Failed to run server: {:?}", e)
        }
    });

    handle_pr_task(ctx.clone(), rx).await;
    // pr_handler.join().expect("oops! pr task thread panicked");
}

