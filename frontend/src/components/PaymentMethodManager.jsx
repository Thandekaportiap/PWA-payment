import React, { useState, useEffect } from 'react';
import { getUserPaymentMethods, createRecurringPayment, deactivatePaymentMethod } from '../services/api';

const PaymentMethodManager = ({ user, subscription, onRecurringPaymentSuccess }) => {
  const [paymentMethods, setPaymentMethods] = useState([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [processingPayment, setProcessingPayment] = useState(null);

  useEffect(() => {
    if (user && user.id) {
      loadPaymentMethods();
    }
  }, [user]);

  const loadPaymentMethods = async () => {
    setLoading(true);
    setError('');
    try {
      const response = await getUserPaymentMethods(user.id);
      setPaymentMethods(response.payment_methods || []);
    } catch (err) {
      console.error('Failed to load payment methods:', err);
      setError(err.message || 'Failed to load payment methods');
    } finally {
      setLoading(false);
    }
  };

  const handleRecurringPayment = async (paymentMethodId) => {
    if (!subscription || !subscription.id) {
      setError('No active subscription found');
      return;
    }

    setProcessingPayment(paymentMethodId);
    setError('');

    try {
      const recurringData = {
        user_id: user.id,
        subscription_id: subscription.id,
        amount: 100.0, // R100 monthly subscription
        payment_method_detail_id: paymentMethodId,
      };

      const result = await createRecurringPayment(recurringData);
      
      if (result.status === 'Completed') {
        if (onRecurringPaymentSuccess) {
          onRecurringPaymentSuccess(result);
        }
      } else {
        setError(`Payment failed: ${result.status}`);
      }
    } catch (err) {
      console.error('Recurring payment failed:', err);
      setError(err.message || 'Recurring payment failed');
    } finally {
      setProcessingPayment(null);
    }
  };

  const handleDeactivatePaymentMethod = async (paymentMethodId) => {
    if (!confirm('Are you sure you want to remove this payment method?')) {
      return;
    }

    try {
      await deactivatePaymentMethod(user.id, paymentMethodId);
      // Reload payment methods to reflect the change
      await loadPaymentMethods();
    } catch (err) {
      console.error('Failed to deactivate payment method:', err);
      setError(err.message || 'Failed to remove payment method');
    }
  };

  const getPaymentMethodDisplay = (method) => {
    switch (method.payment_method) {
      case 'Card':
        return {
          icon: '💳',
          title: method.card_brand ? `${method.card_brand} Card` : 'Credit/Debit Card',
          subtitle: method.card_last_four ? `****${method.card_last_four}` : 'Card payment',
          expiry: method.expiry_month && method.expiry_year ? 
            `Expires ${method.expiry_month.toString().padStart(2, '0')}/${method.expiry_year}` : null
        };
      case 'EFT':
        return {
          icon: '🏦',
          title: 'Instant EFT',
          subtitle: method.bank_name || 'Bank transfer',
          expiry: null
        };
      case 'ScanToPay':
        return {
          icon: '📱',
          title: 'Scan to Pay',
          subtitle: 'Mobile payment',
          expiry: null
        };
      default:
        return {
          icon: '💰',
          title: 'Payment Method',
          subtitle: method.payment_method,
          expiry: null
        };
    }
  };

  if (loading) {
    return (
      <div className="payment-methods-container">
        <h3>💳 Saved Payment Methods</h3>
        <div className="loading-message">
          <span className="spinner"></span>
          Loading payment methods...
        </div>
      </div>
    );
  }

  return (
    <div className="payment-methods-container">
      <h3>💳 Saved Payment Methods</h3>
      
      {error && <div className="error-message">{error}</div>}
      
      {paymentMethods.length === 0 ? (
        <div className="info-message">
          <p>No saved payment methods found.</p>
          <p>Complete a payment to automatically save your payment method for future use.</p>
        </div>
      ) : (
        <div className="payment-methods-list">
          {paymentMethods.map((method) => {
            const display = getPaymentMethodDisplay(method);
            const isProcessing = processingPayment === method.id;
            
            return (
              <div key={method.id} className={`payment-method-card ${method.is_default ? 'default' : ''}`}>
                <div className="payment-method-info">
                  <div className="payment-method-icon">{display.icon}</div>
                  <div className="payment-method-details">
                    <div className="payment-method-title">
                      {display.title}
                      {method.is_default && <span className="default-badge">Default</span>}
                    </div>
                    <div className="payment-method-subtitle">{display.subtitle}</div>
                    {display.expiry && (
                      <div className="payment-method-expiry">{display.expiry}</div>
                    )}
                  </div>
                </div>
                
                <div className="payment-method-actions">
                  <button
                    onClick={() => handleRecurringPayment(method.id)}
                    disabled={isProcessing || !method.peach_registration_id}
                    className="pay-button"
                    title={!method.peach_registration_id ? 'This payment method does not support recurring payments' : ''}
                  >
                    {isProcessing ? (
                      <>
                        <span className="spinner"></span>
                        Processing...
                      </>
                    ) : (
                      'Pay R100.00'
                    )}
                  </button>
                  
                  <button
                    onClick={() => handleDeactivatePaymentMethod(method.id)}
                    className="remove-button"
                    disabled={isProcessing}
                  >
                    Remove
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      )}
      
      <div className="payment-methods-info">
        <p><strong>Recurring Payments:</strong> Use your saved payment methods for quick subscription renewals.</p>
        <p><strong>Security:</strong> We securely store only the necessary payment information. Full card details are never stored.</p>
      </div>
    </div>
  );
};

export default PaymentMethodManager;