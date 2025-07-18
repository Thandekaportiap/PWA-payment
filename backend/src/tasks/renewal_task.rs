use std::sync::Arc;
use chrono::Utc;
use tokio::time::{sleep, Duration as TokioDuration};
use crate::services::database::DatabaseService;
use crate::services::peach::PeachPaymentService;
use crate::models::subscription::SubscriptionStatus;
use crate::models::payment::{PaymentMethod, CreatePaymentDto};

pub async fn start_renewal_task(
    db: Arc<DatabaseService>,
    peach: Arc<PeachPaymentService>,
) {
    tokio::spawn(async move {
        loop {
            println!("⏰ Running renewal task at {}", Utc::now());
            
            // Get subscriptions due for renewal
            let due_subs = match db.get_due_subscriptions().await {  // ✅ Added .await
                Ok(list) => list,
                Err(e) => {
                    eprintln!("⚠️ Error fetching due subscriptions: {}", e);
                    vec![]
                }
            };
            
            for sub in due_subs {
                let user_id = sub.user_id;
                let sub_id = sub.id;
                let token_opt = db.get_recurring_token_by_user(&user_id).await;
                
                match token_opt {
                    Some(token) => {
                        // Automatically charge
                        println!("💳 Attempting auto-debit for sub {} with token {}", sub_id, token);
                        
                        let transaction_id = format!("RENEWAL_{}", uuid::Uuid::new_v4().simple());
                        let charge_result = peach
                            .execute_recurring_payment(&token, sub.price, &transaction_id)
                            .await;
                        
                        match charge_result {
                            Ok(response) => {
                                // Check if the payment was actually successful
                                let result_code = response
                                    .get("result")
                                    .and_then(|r| r.get("code"))
                                    .and_then(|c| c.as_str())
                                    .unwrap_or_default();
                                
                                if result_code.starts_with("000.000") || result_code.starts_with("000.100") {
                                    // Payment successful
                                    if let Err(e) = db.mark_subscription_renewed(&sub_id).await {  // ✅ Added .await
                                        eprintln!("❌ Failed to mark subscription {} as renewed: {}", sub_id, e);
                                    } else {
                                        println!("✅ Auto-renewal succeeded for sub {}", sub_id);
                                    }
                                } else {
                                    eprintln!("❌ Auto-renewal payment failed for sub {}: {}", sub_id, result_code);
                                    // Send manual renewal notification
                                    if let Err(e) = db.create_manual_renewal_notification(user_id, sub_id).await {  // ✅ Added .await
                                        eprintln!("❌ Failed to create renewal notification: {}", e);
                                    }
                                }
                            }
                            Err(err) => {
                                eprintln!("❌ Auto-renewal failed for sub {}: {}", sub_id, err);
                                // Send manual renewal notification
                                if let Err(e) = db.create_manual_renewal_notification(user_id, sub_id).await {  // ✅ Added .await
                                    eprintln!("❌ Failed to create renewal notification: {}", e);
                                }
                            }
                        }
                    }
                    None => {
                        // No recurring token found - check payment method
                        let method = sub.payment_method.clone().unwrap_or(PaymentMethod::Card);
                        if method != PaymentMethod::Card {
                            println!("📣 No token found for manual method {:?}. Sending reminder.", method);
                        } else {
                            println!("⚠️ No token found for CARD method. Cannot auto-renew for sub {}", sub_id);
                        }
                        
                        // Send manual renewal notification regardless of method
                        if let Err(e) = db.create_manual_renewal_notification(user_id, sub_id).await {  // ✅ Added .await
                            eprintln!("❌ Failed to create renewal notification: {}", e);
                        }
                    }
                }
            }
            
            // Suspend expired subscriptions with 3+ days grace period
            let expired = db.get_expired_unpaid_subscriptions().await.unwrap_or_default();  // ✅ Added .await
            for sub in expired {
                if let Err(e) = db.suspend_subscription(&sub.id).await {  // ✅ Added .await
                    eprintln!("❌ Failed to suspend expired subscription {}: {}", sub.id, e);
                } else {
                    println!("🛑 Suspended expired subscription: {}", sub.id);
                }
            }
            
            // Wait 5 minutes for testing (change to 24 hours in production)
            sleep(TokioDuration::from_secs(60 * 5)).await;
            // For production, use: sleep(TokioDuration::from_secs(60 * 60 * 24)).await;
        }
    });
}
