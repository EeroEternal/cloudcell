use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use cloudcell::config::Config;
use cloudcell::db;
use cloudcell::mail::Mailer;
use cloudcell::server::create_router;
use cloudcell::state::AppState;
use http_body_util::BodyExt;
use tower::ServiceExt;

async fn harness() -> (axum::Router, Mailer) {
    let pool = db::connect("sqlite::memory:")
        .await
        .expect("connect in-memory sqlite");
    let mailer = Mailer::log();
    let state = AppState::new(Config::default(), pool).with_mailer(mailer.clone());
    (create_router(state), mailer)
}

async fn app() -> axum::Router {
    harness().await.0
}

async fn json_body(response: axum::http::Response<Body>) -> serde_json::Value {
    let body = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).unwrap()
}

fn last_code(mailer: &Mailer) -> String {
    mailer
        .last_code
        .lock()
        .unwrap()
        .as_ref()
        .expect("code sent")
        .1
        .clone()
}

async fn register_session(app: &axum::Router, mailer: &Mailer) -> String {
    register_named(app, mailer, "ci@cloudcell.dev").await
}

async fn register_named(app: &axum::Router, mailer: &Mailer, email: &str) -> String {
    let send = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/send-code")
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"email":"{email}"}}"#)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(send.status(), StatusCode::OK);
    let code = last_code(mailer);
    let verify = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/verify-code")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"email":"{email}","code":"{code}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(verify.status(), StatusCode::OK);
    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"email":"{email}","code":"{code}","password":"password1"}}"#
                )))
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
    let (app, mailer) = harness().await;
    let key = register_session(&app, &mailer).await;

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
    let (app, mailer) = harness().await;
    let session = register_session(&app, &mailer).await;
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
    let (app, mailer) = harness().await;
    let session = register_session(&app, &mailer).await;
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
                .uri("/api/v1/auth/send-code")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"email":"two@cloudcell.dev"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_snapshot_catalog_requires_auth() {
    let (app, mailer) = harness().await;
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

    let key = register_session(&app, &mailer).await;
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
async fn test_sandboxes_are_per_user() {
    let (app, mailer) = harness().await;
    let alice = register_named(&app, &mailer, "alice@cloudcell.dev").await;
    let bob = register_named(&app, &mailer, "bob@cloudcell.dev").await;

    let created = app
        .clone()
        .oneshot(json_req(
            "POST",
            "/api/v1/sandboxes",
            &alice,
            r#"{"snapshot":"base"}"#,
        ))
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::OK);
    let id = json_body(created).await["id"].as_str().unwrap().to_string();

    let bobs_list = app
        .clone()
        .oneshot(get_req("/api/v1/sandboxes", &bob))
        .await
        .unwrap();
    assert_eq!(json_body(bobs_list).await.as_array().unwrap().len(), 0);

    let bobs_get = app
        .clone()
        .oneshot(get_req(&format!("/api/v1/sandboxes/{id}"), &bob))
        .await
        .unwrap();
    assert_eq!(bobs_get.status(), StatusCode::NOT_FOUND);

    let bobs_exec = app
        .clone()
        .oneshot(json_req(
            "POST",
            &format!("/api/v1/sandboxes/{id}/exec"),
            &bob,
            r#"{"argv":["true"]}"#,
        ))
        .await
        .unwrap();
    assert_eq!(bobs_exec.status(), StatusCode::NOT_FOUND);

    let bobs_del = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/sandboxes/{id}"))
                .header("authorization", format!("Bearer {bob}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bobs_del.status(), StatusCode::NOT_FOUND);

    let alice_list = app
        .oneshot(get_req("/api/v1/sandboxes", &alice))
        .await
        .unwrap();
    assert_eq!(json_body(alice_list).await.as_array().unwrap().len(), 1);
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
        let mailer = Mailer::log();
        let app = create_router(AppState::new(Config::default(), pool).with_mailer(mailer.clone()));
        key = register_session(&app, &mailer).await;
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
