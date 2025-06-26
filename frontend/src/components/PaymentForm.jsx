// frontend/src/components/PaymentForm.jsx
import React, { useState } from 'react';
import { initiatePayment, pollPaymentStatus, processVoucher, createSubscription } from '../services/api';
import PeachCheckout from './PeachCheckout';
import VoucherInput from './VoucherInput';

const PaymentForm = ({ user, onPaymentSuccess }) => {
  const [paymentMethod, setPaymentMethod] = useState('card');
  const [voucherPin, setVoucherPin] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [checkoutForm, setCheckoutForm] = useState(null); // Will hold { action_url, method, fields }
  const [eftDetails, setEftDetails] = useState(null);
  const [paymentResult, setPaymentResult] = useState(null);

  const handleSubmit = async (e) => {
    e.preventDefault();
    setLoading(true);
    setError('');
    setCheckoutForm(null); // Clear any previous checkout form
    setEftDetails(null);
    setPaymentResult(null);

    if (!user || !user.id) {
      setError('User not identified. Please go back and create your account.');
      setLoading(false);
      return;
    }

    // Handle 1Voucher payment directly
    if (paymentMethod === 'onevoucher') {
      if (!voucherPin || voucherPin.length !== 16) {
        setError('Please enter a valid 16-digit 1Voucher PIN');
        setLoading(false);
        return;
      }
      try {
        const voucherPayload = {
          user_id: user.id,
          voucher_code: voucherPin,
        };
        const result = await processVoucher(voucherPayload);
        setPaymentResult({ status: 'completed', message: 'Voucher applied successfully!' });
        onPaymentSuccess({ payment_id: 'voucher-applied', user_id: user.id }); // Simulate payment success
      } catch (err) {
        console.error('1Voucher payment failed:', err);
        setError(err.message || 'Failed to apply voucher.');
      } finally {
        setLoading(false);
      }
      return; // Exit function after handling voucher
    }

    try {
      const paymentData = {
        user_id: user.id, // User ID from the App component
        amount: 100.0, // R100 monthly subscription
        currency: 'ZAR',
        payment_type: paymentMethod.toUpperCase(), // CARD, EFT
      };

      const initiationResponse = await initiatePayment(paymentData);
      console.log('Payment initiated:', initiationResponse);

      if (paymentMethod === 'card') {
        // Correctly use checkout_form_action and checkout_form_fields from the backend response
        const { checkout_form_action, checkout_form_fields } = initiationResponse;

        if (checkout_form_action && checkout_form_fields) {
          setCheckoutForm({
            action_url: checkout_form_action,
            method: 'POST', // Peach Payments form submission is via POST
            fields: checkout_form_fields,
          });
          // PeachCheckout component will now take over and auto-submit the form
        } else {
          setError('Failed to get complete checkout form data from Peach Payments.');
        }

      } else if (paymentMethod === 'eft') {
        // For EFT, show bank details to the user and poll
        setEftDetails({
          bankName: 'Example Bank', // Replace with actual EFT details if your backend provides them
          accountNumber: '1234567890',
          branchCode: '123456',
          reference: initiationResponse.merchant_transaction_id,
          amount: paymentData.amount,
        });
        setPaymentResult({ status: 'pending', message: 'Please complete EFT payment.' });

        // Start polling for EFT payment status
        pollPaymentStatus(initiationResponse.payment_id, initiationResponse.merchant_transaction_id)
          .then(status => {
            console.log('EFT Polling Result:', status);
            if (status.status === 'Completed') {
              onPaymentSuccess({ payment_id: initiationResponse.payment_id, user_id: user.id });
            } else {
              setError(`EFT Payment Status: ${status.status}`);
            }
          })
          .catch(err => {
            console.error('EFT Polling Error:', err);
            setError('Failed to confirm EFT payment status.');
          });
      }
    } catch (err) {
      console.error('Payment initiation failed:', err);
      setError(err.message || 'Failed to initiate payment.');
    } finally {
      setLoading(false);
    }
  };

  const handlePeachCheckoutSuccess = async (peachResult) => {
    // This is called when PeachCheckout indicates a successful redirection (not necessarily payment completion)
    // The actual payment status is confirmed by polling your backend
    console.log('Peach Checkout Redirection Success (from frontend PeachCheckout component):', peachResult);
    setPaymentResult({ status: 'pending', message: 'Redirected to Peach Payments, awaiting confirmation...' });

    // Poll your backend for the final status using the IDs you sent to Peach
    try {
      const finalStatus = await pollPaymentStatus(peachResult.paymentId, peachResult.merchantTransactionId); // Ensure PeachCheckout passes these
      console.log('Final Payment Status from Polling Backend:', finalStatus);
      if (finalStatus.status === 'completed' || finalStatus.status === 'Completed') { // Check both cases
        setPaymentResult({ status: 'completed', message: 'Payment confirmed successfully!' });
        onPaymentSuccess({ payment_id: finalStatus.id, user_id: user.id, merchant_transaction_id: finalStatus.merchant_transaction_id });
      } else {
        setPaymentResult({ status: 'failed', message: `Payment not completed: ${finalStatus.status}. Please try again.` });
        setError(`Payment not completed: ${finalStatus.status}`);
      }
    } catch (err) {
      console.error('Failed to confirm payment via polling:', err);
      setPaymentResult({ status: 'failed', message: 'Payment initiated but failed to get final confirmation.' });
      setError('Payment initiated but failed to get final confirmation.');
    } finally {
        setLoading(false); // Ensure loading is off after polling
    }
  };

  const handlePeachCheckoutError = (err) => {
    setError(err.message || 'Peach Payments checkout failed.');
    setLoading(false);
    setCheckoutForm(null); // Clear checkout form to allow user to retry
    setPaymentResult({ status: 'failed', message: 'Peach Payments redirection failed.' });
  };

  const handleReset = () => {
    setPaymentMethod('card');
    setVoucherPin('');
    setLoading(false);
    setError('');
    setCheckoutForm(null);
    setEftDetails(null);
    setPaymentResult(null);
  };

  // If a PeachCheckout form is generated, render it
  if (checkoutForm) {
    return (
      <PeachCheckout
        checkoutForm={checkoutForm}
        onSuccess={handlePeachCheckoutSuccess}
        onError={handlePeachCheckoutError}
        onCancel={handleReset}
      />
    );
  }

  return (
    <div className="payment-form-container">
      {error && <div className="error-message">{error}</div>}
      {paymentResult && paymentResult.status === 'completed' && (
        <div className="success-message">
          <p>🎉 {paymentResult.message}</p>
        </div>
      )}
      {paymentResult && paymentResult.status === 'pending' && (
        <div className="info-message">
          <p>⏳ {paymentResult.message}</p>
        </div>
      )}
       {paymentResult && paymentResult.status === 'failed' && (
        <div className="error-message">
          <p>❌ {paymentResult.message}</p>
        </div>
      )}

      {!paymentResult && ( // Only show form if no payment result yet
        <form onSubmit={handleSubmit} className="payment-form">
          <div className="form-group">
            <label htmlFor="paymentMethod">Select Payment Method</label>
            <select
              id="paymentMethod"
              value={paymentMethod}
              onChange={(e) => {
                setPaymentMethod(e.target.value);
                setError(''); // Clear errors when changing method
                setVoucherPin('');
              }}
              disabled={loading}
            >
              <option value="card">Credit/Debit Card</option>
              <option value="eft">Instant EFT</option>
              <option value="onevoucher">1Voucher</option>
            </select>
          </div>

          {paymentMethod === 'card' && (
            <div className="payment-method-form">
              <h4>💳 Card Payment</h4>
              <p className="payment-info">
                You will be securely redirected to the Peach Payments gateway to complete your card transaction.
              </p>
            </div>
          )}

          {paymentMethod === 'eft' && (
            <div className="payment-method-form">
              <h4>🏦 Instant EFT</h4>
              <p className="payment-info">
                You will be redirected to process an Instant EFT. Your subscription will be activated once payment is confirmed.
              </p>
              <div className="eft-info">
                <p>• Payment processing time: 1-2 business days</p>
                <p>• Please use the provided reference number</p>
                <p>• Keep your proof of payment for records</p>
              </div>
            </div>
          )}

          {paymentMethod === 'onevoucher' && (
            <div className="payment-method-form">
              <h4>🎫 1Voucher Payment</h4>
              <p className="payment-info">
                Enter your 16-digit 1Voucher PIN to complete the payment instantly.
              </p>
              <VoucherInput
                value={voucherPin}
                onChange={setVoucherPin}
                disabled={loading}
              />
            </div>
          )}

          <button
            type="submit"
            disabled={loading || (paymentMethod === 'onevoucher' && voucherPin.length !== 16)}
            className="payment-button"
          >
            {loading ? (
              <>
                <span className="spinner"></span>
                Processing...
              </>
            ) : (
              `Pay R100.00 with ${paymentMethod === 'card' ? 'Card' : paymentMethod === 'eft' ? 'EFT' : '1Voucher'}`
            )}
          </button>
        </form>
      )}

      {(error || eftDetails || paymentResult) && (
        <button onClick={handleReset} className="reset-button">
          New Payment / Try Again
        </button>
      )}
    </div>
  );
};

export default PaymentForm;