import React, { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { getSubscriptionStatus } from '../services/api';

const SuccessPage: React.FC = () => {
  const navigate = useNavigate();
  const [userData, setUserData] = useState<any>(null);
  const [subscriptionStatus, setSubscriptionStatus] = useState<string>('pending');

  useEffect(() => {
    const storedData = localStorage.getItem('userData');
    if (storedData) {
      const data = JSON.parse(storedData);
      setUserData(data);
      
      // Check subscription status
      checkSubscriptionStatus(data.userId);
    }
  }, []);

  const checkSubscriptionStatus = async (userId: string) => {
    try {
      const response = await getSubscriptionStatus(userId);
      if (response.subscription) {
        setSubscriptionStatus(response.subscription.status);
      }
    } catch (error) {
      console.error('Error checking subscription status:', error);
    }
  };

  const handleViewStatus = () => {
    if (userData) {
      navigate(`/status/${userData.userId}`);
    }
  };

  const handleGoHome = () => {
    // Clear stored data
    localStorage.removeItem('userData');
    localStorage.removeItem('subscriptionId');
    navigate('/');
  };

  return (
    <div className="container">
      <div className="header">
        <h1>Payment Successful! 🎉</h1>
      </div>

      <div className="payment-form">
        <div className="success-content">
          <div className="success-icon" style={{ fontSize: '4rem', marginBottom: '1rem' }}>
            ✅
          </div>
          
          <h2>Thank you for your subscription!</h2>
          
          {userData && (
            <div className="subscription-summary">
              <h3>Subscription Summary</h3>
              <p><strong>Name:</strong> {userData.name}</p>
              <p><strong>Email:</strong> {userData.email}</p>
              <p><strong>Payment Method:</strong> {userData.paymentMethod}</p>
              <p><strong>Amount:</strong> R100.00 per month</p>
              <p><strong>Status:</strong> <span className={`status-${subscriptionStatus.toLowerCase()}`}>{subscriptionStatus}</span></p>
            </div>
          )}

          <div className="next-steps">
            <h3>What's Next?</h3>
            <ul style={{ textAlign: 'left', maxWidth: '400px', margin: '0 auto' }}>
              <li>Your subscription is now active</li>
              <li>You'll be charged R100 every 30 days</li>
              <li>You can manage your subscription anytime</li>
              {userData?.paymentMethod === 'OneVoucher' && (
                <li><strong>Important:</strong> Remember to add a new voucher before your next billing date</li>
              )}
            </ul>
          </div>

          <div className="action-buttons" style={{ marginTop: '2rem' }}>
            <button onClick={handleViewStatus} style={{ marginRight: '1rem' }}>
              View Subscription Status
            </button>
            <button onClick={handleGoHome} style={{ backgroundColor: '#6c757d' }}>
              Start New Subscription
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};

export default SuccessPage;
