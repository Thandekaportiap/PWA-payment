import React, { useState, useEffect } from 'react';
import PaymentForm from './components/PaymentForm';
import SubscriptionStatus from './components/SubscriptionStatus';
import { createUser, createSubscription, getSubscription, healthCheck, getUser } from './services/api';
import './styles/main.css';

function App() {
  const [currentStep, setCurrentStep] = useState('user-setup');
  const [user, setUser] = useState(null);
  const [subscription, setSubscription] = useState(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [apiHealth, setApiHealth] = useState(null);

  useEffect(() => {
    checkApiHealth();
    // In a non-JWT scenario, user state is lost on refresh.
    // If you want persistence without JWT, you'd save user.id to localStorage
    // and retrieve it here, then call getUser to re-hydrate the user object.
    const storedUserId = localStorage.getItem('user_id');
    if (storedUserId) {
        // Attempt to re-fetch user details if ID is stored
        // This is still not secure authentication, just state persistence.
        fetchUserById(storedUserId);
    }
  }, []);

  const checkApiHealth = async () => {
    try {
      await healthCheck();
      setApiHealth('healthy');
      console.log('✅ API is healthy');
    } catch (error) {
      setApiHealth('unhealthy');
      console.error('❌ API health check failed:', error);
    }
  };

  const fetchUserById = async (userId) => {
    setLoading(true);
    setError('');
    try {
      const fetchedUser = await getUser(userId);
      setUser(fetchedUser);
      // If user exists, try to get their latest subscription
      const userSubscription = await getSubscription(fetchedUser.id);
      setSubscription(userSubscription);
      setCurrentStep('payment'); // Or 'success' if subscription is active
    } catch (err) {
      console.error('❌ Failed to fetch user or subscription by ID:', err);
      // If retrieval fails, clear storage and go back to user setup
      localStorage.removeItem('user_id');
      setUser(null);
      setSubscription(null);
      setCurrentStep('user-setup');
      setError('Could not restore session. Please create a new account.');
    } finally {
      setLoading(false);
    }
  };

  const handleUserSetup = async (userData) => {
    setLoading(true);
    setError('');

    try {
      console.log('👤 Creating user:', userData);
      const createdUser = await createUser(userData);
      setUser(createdUser);
      localStorage.setItem('user_id', createdUser.id); // Store user ID for basic persistence

      // Create subscription for the user immediately after user creation
      // Note: In this flow, subscription is tied to user, not payment yet.
      // The payment then funds this subscription.
      const subscriptionData = {
        user_id: createdUser.id,
        // No payment_id yet as payment hasn't happened.
        // We'll need to link payment to subscription *after* payment succeeds.
        // For now, let's assume subscription is created as "pending"
        // and only activated after payment. This might require backend logic changes.
        // For simplicity, I'll pass dummy payment ID, but a proper flow would be:
        // 1. Create User
        // 2. Initiate Payment (get payment_id)
        // 3. Create Subscription (using payment_id)

        // Given your current backend structure where createSubscription expects payment_id,
        // we'll simulate a pending state or link it after payment.
        // Let's create a "pending" payment record first for the subscription.
      };

      // You would typically initiate payment first, then create subscription with payment_id
      // Let's adjust this flow to match the backend more directly.
      // The frontend currently creates *both* user and subscription before payment.
      // This implies subscription is created as 'pending'.
      // Then, PaymentForm needs to update that existing subscription.

      // Option 1: Create subscription here as PENDING, update after payment
      // For now, sticking to the original logic which passes user_id and then expects PaymentForm to handle.
      // This assumes `createSubscription` itself doesn't need a `payment_id` for initial creation,
      // or we generate a dummy one for its payload, which is not ideal.
      // The backend `create_subscription` handler expects `payment_id`.
      // This means the `createSubscription` call *should* happen AFTER payment initiation.

      // Let's modify App.jsx to reflect a more logical flow:
      // 1. Create User
      // 2. Go to Payment (where payment initiation and subscription creation happens)
      setCurrentStep('payment');

    } catch (err) {
      console.error('❌ User setup error:', err);
      setError(err.message || 'Failed to create user account');
    } finally {
      setLoading(false);
    }
  };

  const handlePaymentSuccess = async (paymentResult) => {
    console.log('✅ Payment successful:', paymentResult);
    
    try {
      // After successful payment, create/activate the subscription
      // paymentResult should contain payment_id and user_id.
      // Assuming your createSubscription endpoint requires payment_id and user_id
      const subscriptionData = {
        user_id: user.id, // User from state
        payment_id: paymentResult.payment_id, // Payment ID from successful payment initiation
        plan_type: 'monthly', // Or whatever plan type is chosen
        amount: 100.0,
        currency: 'ZAR',
      };
      
      console.log('📋 Creating/Activating subscription post-payment:', subscriptionData);
      const createdSubscription = await createSubscription(subscriptionData);
      setSubscription(createdSubscription);
      setCurrentStep('success');

    } catch (err) {
      console.error('❌ Failed to create/activate subscription after payment:', err);
      setError(err.message || 'Payment successful, but failed to activate subscription.');
      // Still show success since payment went through, but show error about subscription
      setCurrentStep('success');
    }
  };

  const handleReset = () => {
    setCurrentStep('user-setup');
    setUser(null);
    setSubscription(null);
    setError('');
    localStorage.removeItem('user_id'); // Clear stored user ID
  };

  const renderUserSetupForm = () => (
    <div className="step-container">
      <h2>👤 User Information</h2>
      {error && <div className="error-message">{error}</div>}
      
      <form onSubmit={(e) => {
        e.preventDefault();
        const formData = new FormData(e.target);
        const userData = {
          name: formData.get('name'),
          email: formData.get('email'),
          phone: formData.get('phone'), // Frontend collects phone, backend user model doesn't store it
        };
        handleUserSetup(userData);
      }}>
        <div className="form-group">
          <label htmlFor="name">Full Name *</label>
          <input
            type="text"
            id="name"
            name="name"
            required
            disabled={loading}
            placeholder="Enter your full name"
          />
        </div>

        <div className="form-group">
          <label htmlFor="email">Email Address *</label>
          <input
            type="email"
            id="email"
            name="email"
            required
            disabled={loading}
            placeholder="Enter your email address"
          />
        </div>

        <div className="form-group">
          <label htmlFor="phone">Phone Number *</label>
          <input
            type="tel"
            id="phone"
            name="phone"
            required
            disabled={loading}
            placeholder="Enter your phone number"
          />
        </div>

        <button type="submit" disabled={loading}>
          {loading ? (
            <>
              <span className="spinner"></span>
              Creating Account...
            </>
          ) : (
            'Continue to Payment'
          )}
        </button>
      </form>
    </div>
  );

  const renderPaymentForm = () => (
    <div className="step-container">
      <h2>💳 Payment</h2>
      {error && <div className="error-message">{error}</div>} {/* Display errors from handlePaymentSuccess */}
      {user ? (
        <PaymentForm
          // Pass user object and the subscription data structure if available
          // PaymentForm will handle initiatePayment and call onPaymentSuccess
          subscription={subscription} // subscription will be null initially here, that's fine
          user={user}
          onPaymentSuccess={handlePaymentSuccess}
        />
      ) : (
        <div className="info-message">Please go back and create your user account first.</div>
      )}
    </div>
  );

  const renderSuccess = () => (
    <div className="step-container">
      <h2>🎉 Success!</h2>
      <div className="success-message">
        <p>Congratulations! Your subscription has been activated successfully.</p>
      </div>
      
      {subscription && user && (
        <SubscriptionStatus
          subscription={subscription}
          user={user}
        />
      )}

      <button onClick={handleReset} className="reset-button">
        Start New Subscription
      </button>
    </div>
  );

  return (
    <div className="app">
      <header className="app-header">
        <h1>🇿🇦 SA Subscription Service</h1>
        <p>Premium Monthly Subscription - R100/month</p>
        {apiHealth && (
          <div className={`api-status ${apiHealth}`}>
            API Status: {apiHealth === 'healthy' ? '✅ Online' : '❌ Offline'}
          </div>
        )}
      </header>

      <main className="app-main">
        {currentStep === 'user-setup' && renderUserSetupForm()}
        {currentStep === 'payment' && renderPaymentForm()}
        {currentStep === 'success' && renderSuccess()}
      </main>

      <footer className="app-footer">
        <p>&copy; 2024 SA Subscription Service</p>
        <p>Powered by <a href="https://www.peachpayments.com" target="_blank" rel="noopener noreferrer">Peach Payments</a></p>
      </footer>
    </div>
  );
}

export default App;