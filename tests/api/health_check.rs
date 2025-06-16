use crate::app::spawn_app;

#[tokio::test]
async fn health_check_works() {
    let test_app = spawn_app().await;

    dbg!(&test_app.address);
    let response = test_app
        .client
        .get(format!("{}/health_check", &test_app.address))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}
