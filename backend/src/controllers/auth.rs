use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use axum_extra::extract::{
    cookie::{Cookie, CookieJar, SameSite},
    Multipart,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{DateTime, TimeZone, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use tokio::fs;
use uuid::Uuid;

use crate::db::AppState;
use crate::models::auth::{AuthPayload, Claims, UserResponse};

const UPLOADS_DIR: &str = "./uploads/avatars";
const MAX_FILE_SIZE: usize = 5 * 1024 * 1024; // 5 MB

// POST /register handler
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<AuthPayload>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Hash the password
    let hashed_password = match hash(&payload.password, DEFAULT_COST) {
        Ok(h) => h,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to hash password: {}", e),
            ))
        }
    };

    // Insert user into database
    let result = sqlx::query_as::<_, (i32, String, String, Option<String>, DateTime<Utc>)>(
        r#"
        INSERT INTO users (username, password)
        VALUES ($1, $2)
        RETURNING id, username, role, avatar_url, created_at
        "#
    )
    .bind(payload.username)
    .bind(hashed_password)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok((id, username, role, avatar_url, created_at)) => {
            let response = UserResponse {
                id,
                username,
                role,
                avatar_url,
                created_at,
            };
            Ok((StatusCode::CREATED, Json(response)))
        }
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
            Err((StatusCode::CONFLICT, "Username already exists".to_string()))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {}", e),
        )),
    }
}

// POST /login handler
pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<AuthPayload>,
) -> impl IntoResponse {
    // Find user in database
    let user = sqlx::query_as::<_, (i32, String, String, String, Option<String>, DateTime<Utc>)>(
        r#"
        SELECT id, username, password, role, avatar_url, created_at
        FROM users
        WHERE username = $1
        "#
    )
    .bind(payload.username)
    .fetch_optional(&state.db)
    .await;

    match user {
        Ok(Some((id, username, password, role, avatar_url, created_at))) => {
            // Verify password
            let is_valid = verify(&payload.password, &password).unwrap_or(false);
            if is_valid {
                // Generate JWT
                let expiration = Utc::now()
                    .checked_add_signed(chrono::Duration::hours(24))
                    .expect("valid timestamp")
                    .timestamp() as usize;

                let claims = Claims {
                    sub: username.clone(),
                    user_id: id,
                    role: role.clone(),
                    exp: expiration,
                };

                let token = match encode(
                    &Header::default(),
                    &claims,
                    &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
                ) {
                    Ok(t) => t,
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("JWT generation failed: {}", e),
                        )
                            .into_response();
                    }
                };

                // Store token in HttpOnly cookie
                let mut cookie = Cookie::new("token", token.clone());
                cookie.set_path("/");
                cookie.set_http_only(true);
                cookie.set_same_site(SameSite::Lax);

                let response = UserResponse {
                    id,
                    username,
                    role,
                    avatar_url,
                    created_at,
                };
                (StatusCode::OK, jar.add(cookie), Json(response)).into_response()
            } else {
                (StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()).into_response()
            }
        }
        Ok(None) => (StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {}", e),
        )
            .into_response(),
    }
}

// GET /me handler (Protected Route)
pub async fn me(
    State(state): State<AppState>,
    jar: CookieJar,
) -> impl IntoResponse {
    // Extract token from cookie
    let token = match jar.get("token") {
        Some(cookie) => cookie.value().to_string(),
        None => {
            println!("DEBUG /me: Missing token in cookie header");
            return (StatusCode::UNAUTHORIZED, "Missing token in cookie".to_string()).into_response();
        }
    };

    // Decode and validate token
    let token_data = match decode::<Claims>(
        &token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &Validation::default(),
    ) {
        Ok(data) => data,
        Err(e) => {
            println!("DEBUG /me: Invalid Token error: {}", e);
            return (StatusCode::UNAUTHORIZED, format!("Invalid Token: {}", e)).into_response();
        }
    };

    // Check if token is blacklisted
    let is_blacklisted = match sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM blacklisted_tokens WHERE token = $1)"
    )
    .bind(&token)
    .fetch_one(&state.db)
    .await {
        Ok(b) => b,
        Err(e) => {
            println!("DEBUG /me: Database error checking blacklist: {}", e);
            false
        }
    };

    if is_blacklisted {
        println!("DEBUG /me: Token is blacklisted");
        return (StatusCode::UNAUTHORIZED, "Token has been invalidated (logged out)".to_string()).into_response();
    }

    let claims = token_data.claims;

    // Fetch user from database
    let user = sqlx::query_as::<_, (i32, String, String, Option<String>, DateTime<Utc>)>(
        r#"
        SELECT id, username, role, avatar_url, created_at
        FROM users
        WHERE id = $1
        "#
    )
    .bind(claims.user_id)
    .fetch_optional(&state.db)
    .await;

    match user {
        Ok(Some((id, username, role, avatar_url, created_at))) => {
            let response = UserResponse {
                id,
                username,
                role,
                avatar_url,
                created_at,
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, "User not found".to_string()).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {}", e),
        )
            .into_response(),
    }
}

// POST /logout handler
pub async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> impl IntoResponse {
    if let Some(cookie) = jar.get("token") {
        let token = cookie.value().to_string();
        
        // Decode the token to get its expiration time
        if let Ok(token_data) = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
            &Validation::default(),
        ) {
            let expires_at = Utc.timestamp_opt(token_data.claims.exp as i64, 0).unwrap();
            
            // Insert the token into the blacklist
            let _ = sqlx::query(
                "INSERT INTO blacklisted_tokens (token, expires_at) VALUES ($1, $2) ON CONFLICT DO NOTHING"
            )
            .bind(&token)
            .bind(expires_at)
            .execute(&state.db)
            .await;
        }
    }

    let mut cookie = Cookie::from("token");
    cookie.set_path("/");
    
    (
        StatusCode::OK,
        jar.remove(cookie),
        "Logout successful. Token completely invalidated on server.",
    ).into_response()
}

// POST /avatar handler (Protected Route) - Upload avatar image
pub async fn upload_avatar(
    State(state): State<AppState>,
    jar: CookieJar,
    mut multipart: Multipart,
) -> impl IntoResponse {
    // Authenticate user
    let token = match jar.get("token") {
        Some(cookie) => cookie.value().to_string(),
        None => {
            return (StatusCode::UNAUTHORIZED, "Missing token in cookie".to_string()).into_response();
        }
    };

    let token_data = match decode::<Claims>(
        &token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &Validation::default(),
    ) {
        Ok(data) => data,
        Err(e) => {
            return (StatusCode::UNAUTHORIZED, format!("Invalid Token: {}", e)).into_response();
        }
    };

    // Check blacklist
    let is_blacklisted = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM blacklisted_tokens WHERE token = $1)"
    )
    .bind(&token)
    .fetch_one(&state.db)
    .await
    .unwrap_or(false);

    if is_blacklisted {
        return (StatusCode::UNAUTHORIZED, "Token has been invalidated".to_string()).into_response();
    }

    let user_id = token_data.claims.user_id;

    // Ensure uploads directory exists
    if let Err(e) = fs::create_dir_all(UPLOADS_DIR).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create uploads directory: {}", e),
        )
            .into_response();
    }

    // Process the multipart field
    while let Ok(Some(field)) = multipart.next_field().await {
        let field_name = field.name().unwrap_or("").to_string();
        if field_name != "avatar" {
            continue;
        }

        // Validate content type
        let content_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();

        let extension = match content_type.as_str() {
            "image/jpeg" | "image/jpg" => "jpg",
            "image/png" => "png",
            "image/webp" => "webp",
            "image/gif" => "gif",
            _ => {
                return (
                    StatusCode::BAD_REQUEST,
                    "Invalid file type. Only JPEG, PNG, WebP, and GIF are allowed.".to_string(),
                )
                    .into_response();
            }
        };

        // Read file bytes
        let data = match field.bytes().await {
            Ok(b) => b,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    format!("Failed to read file: {}", e),
                )
                    .into_response();
            }
        };

        // Validate file size (5 MB max)
        if data.len() > MAX_FILE_SIZE {
            return (
                StatusCode::PAYLOAD_TOO_LARGE,
                "File too large. Maximum size is 5 MB.".to_string(),
            )
                .into_response();
        }

        // Delete old avatar file if exists
        let old_avatar = sqlx::query_scalar::<_, Option<String>>(
            "SELECT avatar_url FROM users WHERE id = $1"
        )
        .bind(user_id)
        .fetch_one(&state.db)
        .await
        .unwrap_or(None);

        if let Some(old_url) = old_avatar {
            // old_url is like "/uploads/avatars/uuid.ext"
            let old_path = format!(".{}", old_url);
            let _ = fs::remove_file(&old_path).await;
        }

        // Generate unique filename
        let filename = format!("{}.{}", Uuid::new_v4(), extension);
        let file_path = format!("{}/{}", UPLOADS_DIR, filename);
        let avatar_url = format!("/uploads/avatars/{}", filename);

        // Write file to disk
        if let Err(e) = fs::write(&file_path, &data).await {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to save file: {}", e),
            )
                .into_response();
        }

        // Update user avatar_url in database
        let result = sqlx::query_as::<_, (i32, String, String, Option<String>, DateTime<Utc>)>(
            r#"
            UPDATE users
            SET avatar_url = $1
            WHERE id = $2
            RETURNING id, username, role, avatar_url, created_at
            "#
        )
        .bind(&avatar_url)
        .bind(user_id)
        .fetch_one(&state.db)
        .await;

        return match result {
            Ok((id, username, role, avatar_url, created_at)) => {
                let response = UserResponse {
                    id,
                    username,
                    role,
                    avatar_url,
                    created_at,
                };
                (StatusCode::OK, Json(response)).into_response()
            }
            Err(e) => {
                // Clean up the written file if DB update fails
                let _ = fs::remove_file(&file_path).await;
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Database error: {}", e),
                )
                    .into_response()
            }
        };
    }

    (
        StatusCode::BAD_REQUEST,
        "No avatar field found in request".to_string(),
    )
        .into_response()
}

// DELETE /avatar handler (Protected Route) - Remove avatar
pub async fn delete_avatar(
    State(state): State<AppState>,
    jar: CookieJar,
) -> impl IntoResponse {
    let token = match jar.get("token") {
        Some(cookie) => cookie.value().to_string(),
        None => {
            return (StatusCode::UNAUTHORIZED, "Missing token in cookie".to_string()).into_response();
        }
    };

    let token_data = match decode::<Claims>(
        &token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &Validation::default(),
    ) {
        Ok(data) => data,
        Err(e) => {
            return (StatusCode::UNAUTHORIZED, format!("Invalid Token: {}", e)).into_response();
        }
    };

    let is_blacklisted = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM blacklisted_tokens WHERE token = $1)"
    )
    .bind(&token)
    .fetch_one(&state.db)
    .await
    .unwrap_or(false);

    if is_blacklisted {
        return (StatusCode::UNAUTHORIZED, "Token has been invalidated".to_string()).into_response();
    }

    let user_id = token_data.claims.user_id;

    // Get old avatar path
    let old_avatar = sqlx::query_scalar::<_, Option<String>>(
        "SELECT avatar_url FROM users WHERE id = $1"
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(None);

    // Delete file from disk
    if let Some(old_url) = old_avatar {
        let old_path = format!(".{}", old_url);
        let _ = fs::remove_file(&old_path).await;
    }

    // Clear avatar_url in database
    let result = sqlx::query_as::<_, (i32, String, String, Option<String>, DateTime<Utc>)>(
        r#"
        UPDATE users
        SET avatar_url = NULL
        WHERE id = $1
        RETURNING id, username, role, avatar_url, created_at
        "#
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok((id, username, role, avatar_url, created_at)) => {
            let response = UserResponse {
                id,
                username,
                role,
                avatar_url,
                created_at,
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {}", e),
        )
            .into_response(),
    }
}
