// frontend/src/components/PeachCheckout.jsx
import React, { useEffect, useRef, useState } from 'react';

const PeachCheckout = ({ checkoutForm, onSuccess, onError, onCancel }) => {
  const formRef = useRef(null);
  const [countdown, setCountdown] = useState(3);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (checkoutForm && formRef.current) {
      setLoading(false);
      // Countdown before auto-submit
      const countdownInterval = setInterval(() => {
        setCountdown(prev => {
          if (prev <= 1) {
            clearInterval(countdownInterval);
            formRef.current.submit(); // Auto-submit the form
            return 0;
          }
          return prev - 1;
        });
      }, 1000);

      // No need for window.addEventListener('message', handleMessage) here for this integration type

      return () => {
        clearInterval(countdownInterval);
        // window.removeEventListener('message', handleMessage); // Removed
      };
    }
  }, [checkoutForm]); // No need for onSuccess, onError in dependencies here as they are not used within this effect anymore

  const handleManualSubmit = () => {
    if (formRef.current) {
      formRef.current.submit();
    }
  };

  if (loading || !checkoutForm) {
    return (
      <div className="peach-checkout-loading">
        <div className="loading-spinner"></div>
        <p>Preparing secure payment form...</p>
      </div>
    );
  }

  return (
    <div className="peach-checkout-container">
      <div className="checkout-info">
        <h3>🔒 Secure Payment</h3>
        <p>You will be redirected to Peach Payments secure checkout page.</p>
        {countdown > 0 && (
          <p className="countdown">Redirecting in {countdown} seconds...</p>
        )}
      </div>

      <form
        ref={formRef}
        action={checkoutForm.action_url}
        method={checkoutForm.method}
        acceptCharset="utf-8"
        className="peach-checkout-form"
      >
        {Object.entries(checkoutForm.fields).map(([key, value]) => (
          <input
            key={key}
            type="hidden"
            name={key}
            value={value}
          />
        ))}
      </form>

      <div className="checkout-actions">
        <button
          type="button"
          onClick={handleManualSubmit}
          className="payment-button"
        >
          Continue to Payment
        </button>
        <button
          type="button"
          onClick={onCancel}
          className="cancel-button"
        >
          Cancel Payment
        </button>
      </div>

      <div className="security-info">
        <p>🛡️ Your payment is secured by Peach Payments.</p>
      </div>
    </div>
  );
};

export default PeachCheckout;