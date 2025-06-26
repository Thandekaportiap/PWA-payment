import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { v4 as uuidv4 } from 'uuid';
import { PaymentMethod } from '../types';

const HomePage: React.FC = () => {
  const navigate = useNavigate();
  const [formData, setFormData] = useState({
    name: '',
    email: '',
    paymentMethod: PaymentMethod.Card,
  });

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) => {
    const { name, value } = e.target;
    setFormData(prev => ({
      ...prev,
      [name]: value,
    }));
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const userId = uuidv4();
    
    // Store user data in localStorage for the payment process
    localStorage.setItem('userData', JSON.stringify({
      ...formData,
      userId,
    }));

    navigate('/payment');
  };

  return (
    <div className="container">
      <div className="header">
        <h1>Monthly Subscription</h1>
        <p>Subscribe for R100 per month</p>
      </div>

      <div className="payment-form">
        <h2>Get Started</h2>
        <form onSubmit={handleSubmit}>
          <div className="form-group">
            <label htmlFor="name">Full Name</label>
            <input
              type="text"
              id="name"
              name="name"
              value={formData.name}
              onChange={handleInputChange}
              required
            />
          </div>

          <div className="form-group">
            <label htmlFor="email">Email Address</label>
            <input
              type="email"
              id="email"
              name="email"
              value={formData.email}
              onChange={handleInputChange}
              required
            />
          </div>

          <div className="form-group">
            <label htmlFor="paymentMethod">Payment Method</label>
            <select
              id="paymentMethod"
              name="paymentMethod"
              value={formData.paymentMethod}
              onChange={handleInputChange}
              required
            >
              <option value={PaymentMethod.Card}>Credit Card</option>
              <option value={PaymentMethod.Debit}>Debit Card</option>
              <option value={PaymentMethod.OneVoucher}>1Voucher</option>
              <option value={PaymentMethod.EFT}>EFT</option>
            </select>
          </div>

          <button type="submit">Continue to Payment</button>
        </form>
      </div>
    </div>
  );
};

export default HomePage;
