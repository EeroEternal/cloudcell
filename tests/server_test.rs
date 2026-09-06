use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use cloudcell::config::Config;
use cloudcell::server::create_router;
use cloudcell::state::AppState;
use http_body_util::BodyExt;
use tower::ServiceExt;

fn app() -> axum::Router {
    create_router(AppState::new(Config::default()))
}

async fn json_body(response: axum::http::Response<Body>) -> serde_json::Value {
    let body = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).unwrap()
}

#[tokio::test]
async fn test_health_check() {
    let response = app()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json = json_body(response).await;
    assert_eq!(json["status"], "ok");
    assert_eq!(json["service"], "cloudcell");
}

#[tokio::test]
async fn test_sandbox_lifecycle_and_exec_not_implemented() {
    let app = app();

    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/sandboxes")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"snapshot":"python-3.12"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::OK);
    let created_json = json_body(created).await;
    assert_eq!(created_json["snapshot"], "python-3.12");
    assert_eq!(created_json["state"], "pending");
    let id = created_json["id"].as_str().unwrap().to_string();

    let listed = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/sandboxes")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(listed.status(), StatusCode::OK);
    let listed_json = json_body(listed).await;
    assert_eq!(listed_json.as_array().unwrap().len(), 1);

    let exec = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/sandboxes/{id}/exec"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"argv":["echo","hi"]}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(exec.status(), StatusCode::NOT_IMPLEMENTED);

    let unknown = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/sandboxes")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"snapshot":"does-not-exist"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unknown.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_api_key_single_reveal() {
    let app = app();

    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/keys")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"ci"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::OK);
    let created_json = json_body(created).await;
    assert!(
        created_json["key"]
            .as_str()
            .unwrap()
            .starts_with("cc_live_")
    );

    let listed = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/keys")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let listed_json = json_body(listed).await;
    assert!(listed_json[0]["key"].is_null());
    assert!(listed_json[0]["hint"].as_str().unwrap().contains("••••"));
    assert_ne!(
        listed_json[0]["hint"].as_str().unwrap(),
        created_json["key"].as_str().unwrap()
    );
}

#[tokio::test]
async fn test_snapshot_catalog() {
    let response = app()
        .oneshot(
            Request::builder()
                .uri("/api/v1/snapshots")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = json_body(response).await;
    assert_eq!(json.as_array().unwrap().len(), 3);
    assert_eq!(json[0]["status"], "declared");
}
