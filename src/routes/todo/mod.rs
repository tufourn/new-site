use std::sync::Arc;

use anyhow::Context;
use askama::Template;
use askama_web::WebTemplate;
use axum::{
    Form, Router,
    extract::{Path, State},
    response::{AppendHeaders, IntoResponse},
    routing::{delete, get},
};
use axum_login::login_required;
use http::StatusCode;
use uuid::Uuid;

use crate::{
    app::{ApiContext, AppRouter},
    auth::{AuthSession, Backend},
};

pub fn router() -> AppRouter {
    Router::new()
        .route("/todo", get(get_todos).post(new_todo))
        .route("/todo/{todo_id}", delete(delete_todo).put(update_todo))
        .route_layer(login_required!(Backend, login_url = "/login"))
}

#[derive(Debug)]
struct Todo {
    todo_id: Uuid,
    todo_content: String,
    is_completed: bool,
}

#[derive(Template, WebTemplate)]
#[template(path = "todo/todos_template.html")]
struct TodoTemplate {
    todos: Vec<Todo>,
}

async fn get_todos(
    State(api_context): State<Arc<ApiContext>>,
    auth_session: AuthSession,
) -> impl IntoResponse {
    let user = match auth_session.user {
        Some(user) => user,
        None => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let user_todos = sqlx::query_as!(
        Todo,
        r#"
        SELECT todo_id, todo_content, is_completed FROM todo AS td
        WHERE td.user_id = $1
        ORDER BY td.created_at DESC
        "#,
        user.user_id()
    )
    .fetch_all(&api_context.db)
    .await
    .context("Failed to get todos");

    if let Ok(todos) = user_todos {
        let todo_template = TodoTemplate { todos };
        todo_template.into_response()
    } else {
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}

#[derive(Debug, serde::Deserialize)]
struct NewTodo {
    todo_content: String,
}

#[derive(Debug, serde::Deserialize)]
struct UpdateTodo {
    is_completed: bool,
}

async fn new_todo(
    State(api_context): State<Arc<ApiContext>>,
    auth_session: AuthSession,
    Form(new_todo): Form<NewTodo>,
) -> impl IntoResponse {
    let user = match auth_session.user {
        Some(user) => user,
        None => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let new_todo = sqlx::query!(
        r#"
        INSERT INTO todo (user_id, todo_content)
        VALUES ($1, $2)
        "#,
        user.user_id(),
        new_todo.todo_content
    )
    .execute(&api_context.db)
    .await
    .context("Failed to add todo");

    match new_todo {
        Ok(_) => (
            StatusCode::CREATED,
            AppendHeaders([("HX-Redirect", "/todo")]),
        )
            .into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn delete_todo(
    State(api_context): State<Arc<ApiContext>>,
    auth_session: AuthSession,
    Path(todo_id): Path<Uuid>,
) -> impl IntoResponse {
    let user = match auth_session.user {
        Some(user) => user,
        None => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let result = sqlx::query!(
        r#"
        DELETE FROM todo
        WHERE todo_id = $1 AND user_id = $2
        "#,
        todo_id,
        user.user_id()
    )
    .execute(&api_context.db)
    .await
    .context("Failed to delete todo");

    if let Ok(query_result) = result {
        if query_result.rows_affected() > 0 {
            (StatusCode::OK, AppendHeaders([("HX-Redirect", "/todo")])).into_response()
        } else {
            StatusCode::NOT_FOUND.into_response()
        }
    } else {
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}

async fn update_todo(
    State(api_context): State<Arc<ApiContext>>,
    auth_session: AuthSession,
    Path(todo_id): Path<Uuid>,
    Form(update_todo): Form<UpdateTodo>,
) -> impl IntoResponse {
    let user = match auth_session.user {
        Some(user) => user,
        None => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let result = sqlx::query!(
        r#"
        UPDATE todo
        SET is_completed = $1
        WHERE todo_id = $2 AND user_id = $3
        "#,
        update_todo.is_completed,
        todo_id,
        user.user_id()
    )
    .execute(&api_context.db)
    .await
    .context("Failed to update todo");

    if let Ok(query_result) = result {
        if query_result.rows_affected() > 0 {
            (StatusCode::OK, AppendHeaders([("HX-Redirect", "/todo")])).into_response()
        } else {
            StatusCode::NOT_FOUND.into_response()
        }
    } else {
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}
