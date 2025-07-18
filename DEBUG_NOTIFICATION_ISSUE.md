# Debugging the Notification Creation Issue

## Problem
The API call `POST /api/v1/notifications/test` returns:
```json
{"error":"Failed to create test notification"}
```

## Root Cause Analysis

The error suggests the database operation in `create_test_notification` is failing. Here are the most likely causes:

### 1. **Database Connection Issue**
The backend may not be able to connect to SurrealDB.

### 2. **Table Schema Issue**
The notification table might not exist or have the wrong schema.

### 3. **UUID Generation Issue**
There might be an issue with UUID generation in the method.

## Step-by-Step Fix

### Step 1: Fix the Dependency Issue First

The `async-graphql` dependency is causing compilation issues. Remove or downgrade it:

**Option A: Remove async-graphql (if not needed)**
Edit `backend/Cargo.toml` and remove any references to `async-graphql`.

**Option B: Downgrade to a stable version**
Edit `backend/Cargo.toml`:
```toml
# Change this line if it exists:
# async-graphql = "7.0.17"
# To:
async-graphql = "6.0"
```

### Step 2: Check the create_test_notification Method

Make sure this method exists in `backend/src/services/database.rs`:

```rust
pub async fn create_test_notification(&self, user_id: String, message: String) -> Result<(), String> {
    use uuid::Uuid;
    use chrono::Utc;
    
    let notification_id = Uuid::new_v4().simple().to_string();
    let now = Utc::now();

    let query = r#"
        CREATE notification SET
            id = $record_id,
            user_id = $user_id,
            subscription_id = "test-subscription",
            message = $message,
            acknowledged = false,
            created_at = $created_at
    "#;

    match self.db
        .query(query)
        .bind(("record_id", notification_id.clone()))
        .bind(("user_id", user_id.clone()))
        .bind(("message", message.clone()))
        .bind(("created_at", now))
        .await 
    {
        Ok(_) => {
            println!("📝 Test notification created for user {}: {}", user_id, message);
            Ok(())
        }
        Err(e) => {
            eprintln!("❌ Database error creating notification: {}", e);
            Err(format!("Database error: {}", e))
        }
    }
}
```

### Step 3: Alternative Simple Test Method

Add this simpler method to test database connectivity:

```rust
pub async fn test_database_connection(&self) -> Result<(), String> {
    let query = "SELECT * FROM users LIMIT 1";
    
    match self.db.query(query).await {
        Ok(_) => {
            println!("✅ Database connection working");
            Ok(())
        }
        Err(e) => {
            eprintln!("❌ Database connection failed: {}", e);
            Err(format!("Database error: {}", e))
        }
    }
}
```

### Step 4: Add a Test Endpoint

Add this endpoint to `backend/src/handlers/notification.rs`:

```rust
#[get("/test-db")]
pub async fn test_database(
    db: Data<DatabaseService>,
) -> Result<HttpResponse> {
    match db.test_database_connection().await {
        Ok(_) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "message": "Database connection successful"
        }))),
        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Database test failed: {}", e)
        })))
    }
}
```

And register it in `backend/src/main.rs`:

```rust
.service(
    web::scope("/notifications")
        .service(handlers::notification::get_notifications)
        .service(handlers::notification::mark_notification_read)
        .service(handlers::notification::create_test_notification)
        .service(handlers::notification::test_database)  // Add this line
)
```

### Step 5: Environment Setup Commands

Run these commands in order:

```bash
# 1. Start SurrealDB (in one terminal)
export PATH=/home/ubuntu/.surrealdb:$PATH
surreal start --log debug --user root --pass root --bind 0.0.0.0:8000 memory

# 2. In another terminal, build and run backend
cd backend
source /usr/local/cargo/env

# Fix cargo issue by updating rustup
rustup update

# Try building
cargo build

# If that fails, create a minimal .env file
echo "PORT=8080" > .env
echo "PEACH_SECRET_KEY=test_secret" >> .env
echo "PEACH_AUTH_SERVICE_URL=https://test.example.com" >> .env
echo "PEACH_CHECKOUT_V2_ENDPOINT=https://test.example.com" >> .env
echo "PEACH_ENTITY_ID_V2=test_entity" >> .env
echo "PEACH_CLIENT_ID=test_client" >> .env
echo "PEACH_CLIENT_SECRET=test_secret" >> .env
echo "PEACH_MERCHANT_ID=test_merchant" >> .env
echo "PEACH_NOTIFICATION_URL=http://127.0.0.1:8080/api/v1/payments/callback" >> .env
echo "PEACH_SHOPPER_RESULT_URL=http://127.0.0.1:8080/payment-result.html" >> .env

# Run the backend
cargo run
```

### Step 6: Test the Fix

```bash
# Test database connection
curl http://127.0.0.1:8080/api/v1/notifications/test-db

# Register a user first
curl -X POST http://127.0.0.1:8080/api/v1/users/register \
  -H "Content-Type: application/json" \
  -d '{"email": "test@example.com", "name": "Test User"}'

# Then test notification creation
curl -X POST http://127.0.0.1:8080/api/v1/notifications/test \
  -H "Content-Type: application/json" \
  -d '{"user_id": "test-user-123", "message": "Hello World!"}'
```

## Quick Alternative: Frontend Test

If the backend is still having issues, you can test the frontend notification display with mock data:

```javascript
// Add this to your browser console on the frontend
const mockNotifications = [
    {
        id: "test-1",
        user_id: "test-user",
        subscription_id: "test-sub",
        message: "Test notification message",
        acknowledged: false,
        created_at: new Date().toISOString()
    }
];

displayNotifications(mockNotifications);
```

## Expected Results

After applying these fixes:
1. `cargo build` should succeed
2. The backend should start without errors
3. The test endpoint should return success
4. Notification creation should work

If you're still having issues, the problem is likely in the database schema initialization or SurrealDB configuration.