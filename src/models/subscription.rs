use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Duration};
use uuid::Uuid;
use rust_decimal::Decimal;
use validator::Validate;

use crate::models::common::SubscriptionPlan;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SubscriptionStatus {
    Pending,
    Active,
    Grace,      // Post-expiration grace period
    Expired,
    Suspended,  // Temporarily paused
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: Uuid,
    pub user_id: Uuid,
    pub plan: SubscriptionPlan,
    pub status: SubscriptionStatus,
    pub price: Decimal,
    pub currency: String,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub grace_end_date: Option<DateTime<Utc>>,
    pub billing_cycle_anchor: Option<DateTime<Utc>>,
    pub renewal_attempts: u32,
    pub max_renewal_attempts: u32,
    pub auto_renew: bool,
    pub pause_duration: Option<Duration>,
    pub paused_at: Option<DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateSubscriptionRequest {
    pub user_id: Uuid,
    pub plan: SubscriptionPlan,
    pub auto_renew: Option<bool>,
    pub billing_cycle_anchor: Option<DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ChangePlanRequest {
    pub new_plan: SubscriptionPlan,
    pub prorate: Option<bool>,
    pub effective_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct ChangeBillingDateRequest {
    pub new_billing_date: DateTime<Utc>,
    pub prorate: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct SubscriptionStatusResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub plan: SubscriptionPlan,
    pub status: SubscriptionStatus,
    pub price: Decimal,
    pub currency: String,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub grace_end_date: Option<DateTime<Utc>>,
    pub days_until_expiry: Option<i64>,
    pub days_until_grace_end: Option<i64>,
    pub auto_renew: bool,
    pub can_renew: bool,
    pub renewal_attempts: u32,
    pub max_renewal_attempts: u32,
    pub is_paused: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct RenewalInfo {
    pub subscription_id: Uuid,
    pub next_billing_date: Option<DateTime<Utc>>,
    pub amount: Decimal,
    pub currency: String,
    pub days_until_renewal: Option<i64>,
    pub auto_renew: bool,
    pub can_renew_manually: bool,
    pub grace_period_active: bool,
    pub grace_days_remaining: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ProrationCalculation {
    pub current_plan_refund: Decimal,
    pub new_plan_charge: Decimal,
    pub net_amount: Decimal,
    pub effective_date: DateTime<Utc>,
    pub days_used: i64,
    pub days_remaining: i64,
}

impl Subscription {
    pub fn new(request: CreateSubscriptionRequest) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id: request.user_id,
            plan: request.plan.clone(),
            status: SubscriptionStatus::Pending,
            price: request.plan.price(),
            currency: "ZAR".to_string(),
            start_date: None,
            end_date: None,
            grace_end_date: None,
            billing_cycle_anchor: request.billing_cycle_anchor,
            renewal_attempts: 0,
            max_renewal_attempts: 5,
            auto_renew: request.auto_renew.unwrap_or(true),
            pause_duration: None,
            paused_at: None,
            metadata: request.metadata,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn activate(&mut self, grace_period_days: u32) {
        let now = Utc::now();
        let start_date = self.billing_cycle_anchor.unwrap_or(now);
        
        self.status = SubscriptionStatus::Active;
        self.start_date = Some(start_date);
        self.end_date = Some(start_date + Duration::days(self.plan.duration_days()));
        self.grace_end_date = Some(
            self.end_date.unwrap() + Duration::days(grace_period_days as i64)
        );
        self.renewal_attempts = 0;
        self.updated_at = now;
    }

    pub fn extend(&mut self, duration_days: i64) {
        if let Some(end_date) = self.end_date {
            self.end_date = Some(end_date + Duration::days(duration_days));
            if let Some(grace_end) = self.grace_end_date {
                self.grace_end_date = Some(grace_end + Duration::days(duration_days));
            }
        }
        self.updated_at = Utc::now();
    }

    pub fn pause(&mut self) -> Result<(), String> {
        if !matches!(self.status, SubscriptionStatus::Active) {
            return Err("Can only pause active subscriptions".to_string());
        }

        self.status = SubscriptionStatus::Suspended;
        self.paused_at = Some(Utc::now());
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn resume(&mut self) -> Result<(), String> {
        if !matches!(self.status, SubscriptionStatus::Suspended) {
            return Err("Can only resume suspended subscriptions".to_string());
        }

        if let Some(paused_at) = self.paused_at {
            let pause_duration = Utc::now().signed_duration_since(paused_at);
            
            // Extend subscription by pause duration
            if let Some(end_date) = self.end_date {
                self.end_date = Some(end_date + pause_duration);
            }
            if let Some(grace_end) = self.grace_end_date {
                self.grace_end_date = Some(grace_end + pause_duration);
            }
        }

        self.status = SubscriptionStatus::Active;
        self.pause_duration = None;
        self.paused_at = None;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn cancel(&mut self) {
        self.status = SubscriptionStatus::Cancelled;
        self.auto_renew = false;
        self.updated_at = Utc::now();
    }

    pub fn change_plan(&mut self, new_plan: SubscriptionPlan, effective_date: Option<DateTime<Utc>>) -> Result<ProrationCalculation, String> {
        if !matches!(self.status, SubscriptionStatus::Active | SubscriptionStatus::Grace) {
            return Err("Can only change plan for active or grace period subscriptions".to_string());
        }

        let effective_date = effective_date.unwrap_or_else(Utc::now);
        let old_plan = self.plan.clone();
        let old_price = old_plan.price();
        let new_price = new_plan.price();

        // Calculate proration
        let days_in_period = old_plan.duration_days();
        let days_used = if let Some(start_date) = self.start_date {
            effective_date.signed_duration_since(start_date).num_days()
        } else {
            0
        };
        let days_remaining = days_in_period - days_used;

        let daily_rate_old = old_price / Decimal::new(days_in_period, 0);
        let daily_rate_new = new_price / Decimal::new(new_plan.duration_days(), 0);

        let current_plan_refund = daily_rate_old * Decimal::new(days_remaining, 0);
        let new_plan_charge = daily_rate_new * Decimal::new(new_plan.duration_days(), 0);
        let net_amount = new_plan_charge - current_plan_refund;

        // Update subscription
        self.plan = new_plan.clone();
        self.price = new_price;
        
        // Recalculate end date based on new plan
        if let Some(start_date) = self.start_date {
            self.end_date = Some(effective_date + Duration::days(new_plan.duration_days()));
            if let Some(grace_end) = self.grace_end_date {
                let grace_days = grace_end.signed_duration_since(
                    start_date + Duration::days(old_plan.duration_days())
                ).num_days();
                self.grace_end_date = Some(self.end_date.unwrap() + Duration::days(grace_days));
            }
        }

        self.updated_at = Utc::now();

        Ok(ProrationCalculation {
            current_plan_refund,
            new_plan_charge,
            net_amount,
            effective_date,
            days_used,
            days_remaining,
        })
    }

    pub fn change_billing_date(&mut self, new_billing_date: DateTime<Utc>) -> Result<ProrationCalculation, String> {
        if !matches!(self.status, SubscriptionStatus::Active) {
            return Err("Can only change billing date for active subscriptions".to_string());
        }

        let current_end = self.end_date.ok_or("No end date set")?;
        let now = Utc::now();
        
        // Calculate proration for date change
        let days_until_current_end = current_end.signed_duration_since(now).num_days();
        let days_until_new_date = new_billing_date.signed_duration_since(now).num_days();
        let day_difference = days_until_new_date - days_until_current_end;

        let daily_rate = self.price / Decimal::new(self.plan.duration_days(), 0);
        let net_amount = daily_rate * Decimal::new(day_difference, 0);

        // Update dates
        self.end_date = Some(new_billing_date);
        if let Some(grace_end) = self.grace_end_date {
            let grace_days = grace_end.signed_duration_since(current_end).num_days();
            self.grace_end_date = Some(new_billing_date + Duration::days(grace_days));
        }
        self.billing_cycle_anchor = Some(new_billing_date);
        self.updated_at = Utc::now();

        Ok(ProrationCalculation {
            current_plan_refund: Decimal::ZERO,
            new_plan_charge: net_amount,
            net_amount,
            effective_date: new_billing_date,
            days_used: days_until_current_end,
            days_remaining: days_until_new_date,
        })
    }

    pub fn increment_renewal_attempt(&mut self) {
        self.renewal_attempts += 1;
        self.updated_at = Utc::now();
    }

    pub fn reset_renewal_attempts(&mut self) {
        self.renewal_attempts = 0;
        self.updated_at = Utc::now();
    }

    pub fn can_renew_manually(&self) -> bool {
        matches!(self.status, SubscriptionStatus::Expired | SubscriptionStatus::Suspended)
    }

    pub fn can_auto_renew(&self) -> bool {
        self.auto_renew && self.renewal_attempts < self.max_renewal_attempts
    }

    pub fn days_until_expiry(&self) -> Option<i64> {
        self.end_date.map(|end| {
            end.signed_duration_since(Utc::now()).num_days()
        })
    }

    pub fn days_until_grace_end(&self) -> Option<i64> {
        self.grace_end_date.map(|grace_end| {
            grace_end.signed_duration_since(Utc::now()).num_days()
        })
    }

    pub fn is_in_grace_period(&self) -> bool {
        matches!(self.status, SubscriptionStatus::Grace) ||
        (matches!(self.status, SubscriptionStatus::Active) && 
         self.end_date.map_or(false, |end| Utc::now() > end))
    }

    pub fn update_status_based_on_dates(&mut self, grace_period_days: u32) {
        let now = Utc::now();
        
        match self.status {
            SubscriptionStatus::Active => {
                if let Some(end_date) = self.end_date {
                    if now > end_date {
                        self.status = SubscriptionStatus::Grace;
                        if self.grace_end_date.is_none() {
                            self.grace_end_date = Some(end_date + Duration::days(grace_period_days as i64));
                        }
                        self.updated_at = now;
                    }
                }
            },
            SubscriptionStatus::Grace => {
                if let Some(grace_end) = self.grace_end_date {
                    if now > grace_end {
                        self.status = SubscriptionStatus::Expired;
                        self.updated_at = now;
                    }
                }
            },
            _ => {}
        }
    }

    pub fn to_status_response(&self) -> SubscriptionStatusResponse {
        SubscriptionStatusResponse {
            id: self.id,
            user_id: self.user_id,
            plan: self.plan.clone(),
            status: self.status.clone(),
            price: self.price,
            currency: self.currency.clone(),
            start_date: self.start_date,
            end_date: self.end_date,
            grace_end_date: self.grace_end_date,
            days_until_expiry: self.days_until_expiry(),
            days_until_grace_end: self.days_until_grace_end(),
            auto_renew: self.auto_renew,
            can_renew: self.can_renew_manually(),
            renewal_attempts: self.renewal_attempts,
            max_renewal_attempts: self.max_renewal_attempts,
            is_paused: matches!(self.status, SubscriptionStatus::Suspended),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    pub fn get_renewal_info(&self) -> RenewalInfo {
        RenewalInfo {
            subscription_id: self.id,
            next_billing_date: self.end_date,
            amount: self.price,
            currency: self.currency.clone(),
            days_until_renewal: self.days_until_expiry(),
            auto_renew: self.auto_renew,
            can_renew_manually: self.can_renew_manually(),
            grace_period_active: self.is_in_grace_period(),
            grace_days_remaining: self.days_until_grace_end(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscription_activation() {
        let request = CreateSubscriptionRequest {
            user_id: Uuid::new_v4(),
            plan: SubscriptionPlan::Monthly,
            auto_renew: Some(true),
            billing_cycle_anchor: None,
            metadata: None,
        };

        let mut subscription = Subscription::new(request);
        assert_eq!(subscription.status, SubscriptionStatus::Pending);

        subscription.activate(7);
        assert_eq!(subscription.status, SubscriptionStatus::Active);
        assert!(subscription.start_date.is_some());
        assert!(subscription.end_date.is_some());
        assert!(subscription.grace_end_date.is_some());
    }

    #[test]
    fn test_subscription_pause_resume() {
        let request = CreateSubscriptionRequest {
            user_id: Uuid::new_v4(),
            plan: SubscriptionPlan::Monthly,
            auto_renew: Some(true),
            billing_cycle_anchor: None,
            metadata: None,
        };

        let mut subscription = Subscription::new(request);
        subscription.activate(7);

        let original_end_date = subscription.end_date.unwrap();

        // Pause subscription
        assert!(subscription.pause().is_ok());
        assert_eq!(subscription.status, SubscriptionStatus::Suspended);
        assert!(subscription.paused_at.is_some());

        // Resume subscription (in real scenario, there would be a time gap)
        assert!(subscription.resume().is_ok());
        assert_eq!(subscription.status, SubscriptionStatus::Active);
        assert!(subscription.end_date.unwrap() >= original_end_date);
    }

    #[test]
    fn test_plan_change() {
        let request = CreateSubscriptionRequest {
            user_id: Uuid::new_v4(),
            plan: SubscriptionPlan::Monthly,
            auto_renew: Some(true),
            billing_cycle_anchor: None,
            metadata: None,
        };

        let mut subscription = Subscription::new(request);
        subscription.activate(7);

        let result = subscription.change_plan(SubscriptionPlan::Annual, None);
        assert!(result.is_ok());
        assert_eq!(subscription.plan, SubscriptionPlan::Annual);
        assert_eq!(subscription.price, SubscriptionPlan::Annual.price());
    }
}