use std::sync::Arc;
use crate::services::database::DatabaseService;
use crate::services::peach::PeachPaymentService;
use crate::models::payment::{PaymentMethod};

pub async fn start_renewal_task(
    db: Arc<DatabaseService>,
    peach: Arc<PeachPaymentService>,
) {
    // Instead of spawning a loop with sleep, just run renewal logic once
    println!("🔁 Running manual renewal task");

    // Get subscriptions due for renewal
    let due_subs = match db.get_due_subscriptions().await {
        Ok(list) => list,
        Err(e) => {
            eprintln!("⚠️ Error fetching due subscriptions: {}", e);
            return;
        }
    };

    for sub in due_subs {
    let user_id = sub.user_id;
    let sub_id = sub.id.clone(); // ✅ clone early
    let subscription_id = sub_id.to_string(); // ✅ convert to String once

    let token_opt = db.get_recurring_token_by_user(&user_id).await;

    match token_opt {
        Some(token) => {
            println!("💳 Attempting auto-debit for sub {} with token {}", subscription_id, token);

            let transaction_id = format!("RENEWAL_{}", uuid::Uuid::new_v4().simple());
            let charge_result = peach
                .execute_recurring_payment(&token, sub.price, &transaction_id)
                .await;

            match charge_result {
                Ok(response) => {
                    let result_code = response
                        .get("result")
                        .and_then(|r| r.get("code"))
                        .and_then(|c| c.as_str())
                        .unwrap_or_default();

                    if result_code.starts_with("000.000") || result_code.starts_with("000.100") {
                        if let Err(e) = db.mark_subscription_renewed(&subscription_id).await {
                            eprintln!("❌ Failed to mark subscription {} as renewed: {}", subscription_id, e);
                        } else {
                            println!("✅ Auto-renewal succeeded for sub {}", subscription_id);
                        }
                    } else {
                        eprintln!("❌ Payment failed for sub {}: {}", subscription_id, result_code);
                        if let Err(e) = db.create_manual_renewal_notification(user_id, subscription_id.clone()).await {
                            eprintln!("❌ Failed to create renewal notification: {}", e);
                        }
                    }
                }
                Err(err) => {
                    eprintln!("❌ Auto-debit failed for sub {}: {}", subscription_id, err);
                    if let Err(e) = db.create_manual_renewal_notification(user_id, subscription_id.clone()).await {
                        eprintln!("❌ Failed to create renewal notification: {}", e);
                    }
                }
            }
        }
        None => {
            let method = sub.payment_method.clone().unwrap_or(PaymentMethod::Card);
            println!("📣 Manual renewal reminder for {:?} method", method);
            if let Err(e) = db.create_manual_renewal_notification(user_id, subscription_id.clone()).await {
                eprintln!("❌ Failed to create renewal notification: {}", e);
            }
        }
    }
}

    // Optional: suspend subscriptions manually via admin/cron
    let expired = db.get_expired_unpaid_subscriptions().await.unwrap_or_default();
    for sub in expired {
        let subscription_id = sub.id.to_string();
       if let Err(e) = db.suspend_subscription(&subscription_id).await {
            eprintln!("❌ Failed to suspend expired subscription {}: {}", sub.id, e);
        } else {
            println!("🛑 Suspended expired subscription: {}", sub.id);
        }
    }
}
