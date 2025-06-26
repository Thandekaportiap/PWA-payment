import React, { useState, useEffect } from 'react';
import { getSubscription, listPayments, formatCurrency, formatDate } from '../services/api';

const SubscriptionStatus = ({ subscription: initialSubscription, user }) => {
  const [subscription, setSubscription] = useState(initialSubscription);
  const [payments, setPayments] = useState([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  // Use useEffect to load/refresh subscription and payments whenever user or initialSubscription changes
  useEffect(() => {
    if (user?.id) { // Fetch by user.id
      refreshSubscription();
      loadPaymentHistory(user.id); // Load payments related to this user
    }
  }, [user?.id, initialSubscription]); // Re-run if user or initialSubscription prop changes

  const loadPaymentHistory = async (userId) => {
    try {
      // Assuming listPayments can filter by user_id or subscription_id
      // For this example, we don't have listPayments by user, so it's a placeholder.
      // In a real app, you'd fetch payments associated with the user's active subscription.
      // const paymentData = await listPayments({ user_id: userId });
      // setPayments(paymentData.payments || []);
      setPayments([]); // Placeholder: No actual listPayments endpoint by user_id yet
    } catch (err) {
      console.error('Failed to load payment history:', err);
    }
  };

  const refreshSubscription = async () => {
    if (!user?.id) {
        setError('No user identified to refresh subscription.');
        return;
    }
    
    setLoading(true);
    setError('');

    try {
      const updated = await getSubscription(user.id); // Fetch by user_id
      setSubscription(updated);
      // await loadPaymentHistory(user.id); // Reload payments after subscription refresh
    } catch (err) {
      setError('Failed to refresh subscription status. (It might not exist or be active yet)');
      console.error('Refresh error:', err);
      setSubscription(null); // Clear subscription if not found
    } finally {
      setLoading(false);
    }
  };

  const getStatusColor = (status) => {
    switch (status) {
      case 'Active': // Match backend enum variants
        return 'green';
      case 'Pending':
        return 'orange';
      case 'Expired':
        return 'red';
      case 'Cancelled':
        return 'gray';
      default:
        return 'gray';
    }
  };

  if (loading) {
    return <div className="loading-spinner"></div>;
  }

  if (error) {
    return <div className="error-message">{error}</div>;
  }

  if (!subscription) {
    return (
      <div className="info-message">
        <p>No active subscription found for this user.</p>
        <button onClick={refreshSubscription} className="check-status-button">
            Try Refreshing Status
        </button>
      </div>
    );
  }

  return (
    <div className="subscription-status-container">
      <h3>Subscription Details</h3>
      <div className="status-item">
        <span className="label">Status:</span>
        <span className={`value status-color-${getStatusColor(subscription.active ? 'Active' : 'Pending')}`}>
            {subscription.active ? 'Active' : 'Pending'} {/* Simplified status */}
        </span>
      </div>
      <div className="status-item">
        <span className="label">Plan:</span>
        <span className="value">{subscription.plan_type}</span>
      </div>
      <div className="status-item">
        <span className="label">Amount:</span>
        <span className="value">{formatCurrency(subscription.amount, subscription.currency)} / month</span>
      </div>
      <div className="status-item">
        <span className="label">Next Billing:</span>
        <span className="value">{formatDate(subscription.next_billing_date)}</span>
      </div>
      <div className="status-item">
        <span className="label">Payment Method:</span>
        <span className="value">{subscription.payment_method}</span>
      </div>
      <div className="status-item">
        <span className="label">Account Email:</span>
        <span className="value">{user?.email || 'N/A'}</span>
      </div>

      {payments.length > 0 && (
        <div className="payment-history">
          <h4>Payment History</h4>
          <div className="payment-list">
            {payments.map((payment) => (
              <div key={payment.id} className="payment-item">
                <div className="payment-details">
                  <div className="payment-amount">
                    {formatCurrency(payment.amount, payment.currency)}
                  </div>
                  <div className="payment-meta">
                    <div className="payment-status">
                      Status: <span className={`status-color-${getStatusColor(payment.status)}`}>
                        {payment.status.toUpperCase()}
                      </span>
                    </div>
                    
                    <div className="payment-date">
                      {formatDate(payment.created_at)}
                    </div>
                  </div>
                  
                  {payment.transaction_id && (
                    <div className="transaction-id">
                      Transaction: {payment.transaction_id}
                    </div>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {subscription.active === false && ( // If not active, show pending message
        <div className="pending-notice">
          <h4>⏳ Subscription Pending</h4>
          <p>
            Your subscription is waiting for payment confirmation. 
            If you've made a payment, it may take a few minutes to process.
          </p>
          <button onClick={refreshSubscription} className="check-status-button">
            Check Status
          </button>
        </div>
      )}

      {/* No expired state logic in this simplified flow, but you can add it */}
      {/* {subscription.status === 'expired' && (
        <div className="expired-notice">
          <h4>⚠️ Subscription Expired</h4>
          <p>
            Your subscription has expired. Renew now to continue enjoying premium features.
          </p>
          <button className="renew-button">
            Renew Subscription
          </button>
        </div>
      )} */}
    </div>
  );
};

export default SubscriptionStatus;