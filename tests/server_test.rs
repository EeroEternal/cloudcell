use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use cloudcell::config::Config;
use cloudcell::db;
use cloudcell::server::create_router;
use cloudcell::state::AppState;
use http_body_util::BodyExt;
use tower::ServiceExt;

async fn app() -> axum::Router {
    let pool = db::connect("sqlite::memory:")
        .await
        .expect("connect in-memory sqlite");
    create_router(AppState::new(Config::default(), pool))
}

async fn json_body(response: axum::http::Response<Body>) -> serde_json::Value {
    let body = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).unwrap()
}

async fn register_session(app: &axum::Router) -> String {
    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"email":"ci@cloudcell.dev","password":"password1"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::OK);
    json_body(created).await["token"]
        .as_str()
        .unwrap()
        .to_string()
}

fn json_req(method: &str, uri: &str, key: &str, body: &'static str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {key}"))
        .body(Body::from(body))
        .unwrap()
}

fn get_req(uri: &str, key: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .header("authorization", format!("Bearer {key}"))
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn test_health_check() {
    let response = app()
        .await
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
async fn test_sandbox_requires_auth() {
    let app = app().await;
    let unauth = app
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
    assert_eq!(unauth.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_sandbox_lifecycle_and_exec_not_implemented() {
    let app = app().await;
    let key = register_session(&app).await;

    let created = app
        .clone()
        .oneshot(json_req(
            "POST",
            "/api/v1/sandboxes",
            &key,
            r#"{"snapshot":"python-3.12"}"#,
        ))
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::OK);
    let created_json = json_body(created).await;
    assert_eq!(created_json["snapshot"], "python-3.12");
    assert_eq!(created_json["state"], "pending");
    let id = created_json["id"].as_str().unwrap().to_string();

    let listed = app
        .clone()
        .oneshot(get_req("/api/v1/sandboxes", &key))
        .await
        .unwrap();
    assert_eq!(listed.status(), StatusCode::OK);
    let listed_json = json_body(listed).await;
    assert_eq!(listed_json.as_array().unwrap().len(), 1);

    let exec = app
        .clone()
        .oneshot(json_req(
            "POST",
            &format!("/api/v1/sandboxes/{id}/exec"),
            &key,
            r#"{"argv":["echo","hi"]}"#,
        ))
        .await
        .unwrap();
    assert_eq!(exec.status(), StatusCode::NOT_IMPLEMENTED);

    let unknown = app
        .clone()
        .oneshot(json_req(
            "POST",
            "/api/v1/sandboxes",
            &key,
            r#"{"snapshot":"does-not-exist"}"#,
        ))
        .await
        .unwrap();
    assert_eq!(unknown.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_register_login_and_api_key() {
    let app = app().await;
    let session = register_session(&app).await;
    assert!(session.starts_with("cc_sess_"));

    let unauth = app
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
    assert_eq!(unauth.status(), StatusCode::UNAUTHORIZED);

    let created = app
        .clone()
        .oneshot(json_req(
            "POST",
            "/api/v1/keys",
            &session,
            r#"{"name":"ci"}"#,
        ))
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
        .oneshot(get_req("/api/v1/keys", &session))
        .await
        .unwrap();
    let listed_json = json_body(listed).await;
    assert!(listed_json[0]["key"].is_null());
}

#[tokio::test]
async fn test_registration_can_be_disabled() {
    let app = app().await;
    let session = register_session(&app).await;
    let disabled = app
        .clone()
        .oneshot(json_req(
            "PUT",
            "/api/v1/settings",
            &session,
            r#"{"registration_enabled":false}"#,
        ))
        .await
        .unwrap();
    assert_eq!(disabled.status(), StatusCode::OK);

    let second = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"email":"two@cloudcell.dev","password":"password1"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_snapshot_catalog_requires_auth() {
    let app = app().await;
    let denied = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/snapshots")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);

    let key = register_session(&app).await;
    let response = app
        .oneshot(get_req("/api/v1/snapshots", &key))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = json_body(response).await;
    assert_eq!(json.as_array().unwrap().len(), 3);
    assert_eq!(json[0]["status"], "declared");
}

#[tokio::test]
async fn test_sqlite_persists_across_pools() {
    let path = std::env::temp_dir().join(format!(
        "cloudcell-test-{}.db",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let url = format!("sqlite:{}", path.display());
    let key;
    let sandbox_id;

    {
        let pool = db::connect(&url).await.unwrap();
        let app = create_router(AppState::new(Config::default(), pool));
        key = register_session(&app).await;
        let created = app
            .oneshot(json_req(
                "POST",
                "/api/v1/sandboxes",
                &key,
                r#"{"snapshot":"base"}"#,
            ))
            .await
            .unwrap();
        sandbox_id = json_body(created).await["id"].as_str().unwrap().to_string();
    }

    let pool = db::connect(&url).await.unwrap();
    let app = create_router(AppState::new(Config::default(), pool));
    let listed = app
        .oneshot(get_req("/api/v1/sandboxes", &key))
        .await
        .unwrap();
    let listed_json = json_body(listed).await;
    assert_eq!(listed_json[0]["id"], sandbox_id);
    let _ = std::fs::remove_file(&path);
}
