import React, { useState, useEffect } from 'react';
import { listPayments, getSubscription } from '../services/api';

const Statistics = ({ user }) => {
  const [stats, setStats] = useState({
    totalPayments: 0,
    successfulPayments: 0,
    failedPayments: 0,
    totalAmount: 0,
    subscriptionStatus: 'inactive',
    lastPaymentDate: null
  });
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  useEffect(() => {
    if (user?.id) {
      loadStatistics();
    }
  }, [user?.id]);

  const loadStatistics = async () => {
    setLoading(true);
    setError('');

    try {
      // Load payments data
      const paymentsResponse = await listPayments();
      const payments = paymentsResponse.payments || [];
      
      // Filter payments by user if needed
      const userPayments = payments.filter(payment => payment.user_id === user.id);
      
      // Calculate statistics
      const totalPayments = userPayments.length;
      const successfulPayments = userPayments.filter(p => p.status === 'Completed').length;
      const failedPayments = userPayments.filter(p => p.status === 'Failed').length;
      const totalAmount = userPayments
        .filter(p => p.status === 'Completed')
        .reduce((sum, p) => sum + (p.amount || 0), 0);
      
      const lastPayment = userPayments
        .sort((a, b) => new Date(b.created_at) - new Date(a.created_at))[0];

      // Load subscription status
      let subscriptionStatus = 'inactive';
      try {
        const subscription = await getSubscription(user.id);
        subscriptionStatus = subscription?.status || 'inactive';
      } catch (err) {
        console.warn('No subscription found for user');
      }

      setStats({
        totalPayments,
        successfulPayments,
        failedPayments,
        totalAmount,
        subscriptionStatus,
        lastPaymentDate: lastPayment?.created_at || null
      });

    } catch (err) {
      setError('Failed to load statistics');
      console.error('Statistics loading error:', err);
    } finally {
      setLoading(false);
    }
  };

  const formatCurrency = (amount) => {
    return new Intl.NumberFormat('en-ZA', {
      style: 'currency',
      currency: 'ZAR'
    }).format(amount);
  };

  const formatDate = (dateString) => {
    if (!dateString) return 'Never';
    return new Date(dateString).toLocaleDateString('en-ZA', {
      year: 'numeric',
      month: 'short',
      day: 'numeric'
    });
  };

  if (loading) {
    return (
      <div className="statistics-container">
        <h2>Statistics</h2>
        <div className="loading">Loading statistics...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="statistics-container">
        <h2>Statistics</h2>
        <div className="error">{error}</div>
        <button onClick={loadStatistics}>Retry</button>
      </div>
    );
  }

  return (
    <div className="statistics-container">
      <h2>Payment Statistics</h2>
      
      <div className="stats-grid">
        <div className="stat-card">
          <h3>Total Payments</h3>
          <div className="stat-value">{stats.totalPayments}</div>
        </div>

        <div className="stat-card success">
          <h3>Successful Payments</h3>
          <div className="stat-value">{stats.successfulPayments}</div>
        </div>

        <div className="stat-card failed">
          <h3>Failed Payments</h3>
          <div className="stat-value">{stats.failedPayments}</div>
        </div>

        <div className="stat-card amount">
          <h3>Total Amount</h3>
          <div className="stat-value">{formatCurrency(stats.totalAmount)}</div>
        </div>

        <div className="stat-card subscription">
          <h3>Subscription Status</h3>
          <div className={`stat-value status-${stats.subscriptionStatus}`}>
            {stats.subscriptionStatus.charAt(0).toUpperCase() + stats.subscriptionStatus.slice(1)}
          </div>
        </div>

        <div className="stat-card">
          <h3>Last Payment</h3>
          <div className="stat-value">{formatDate(stats.lastPaymentDate)}</div>
        </div>
      </div>

      <div className="statistics-actions">
        <button onClick={loadStatistics} className="refresh-btn">
          Refresh Statistics
        </button>
      </div>

      <style jsx>{`
        .statistics-container {
          padding: 20px;
          max-width: 1200px;
          margin: 0 auto;
        }

        .stats-grid {
          display: grid;
          grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
          gap: 20px;
          margin: 20px 0;
        }

        .stat-card {
          background: white;
          border-radius: 8px;
          padding: 20px;
          box-shadow: 0 2px 4px rgba(0,0,0,0.1);
          border-left: 4px solid #007bff;
        }

        .stat-card.success {
          border-left-color: #28a745;
        }

        .stat-card.failed {
          border-left-color: #dc3545;
        }

        .stat-card.amount {
          border-left-color: #ffc107;
        }

        .stat-card.subscription {
          border-left-color: #6f42c1;
        }

        .stat-card h3 {
          margin: 0 0 10px 0;
          color: #666;
          font-size: 14px;
          font-weight: 500;
        }

        .stat-value {
          font-size: 24px;
          font-weight: bold;
          color: #333;
        }

        .status-active {
          color: #28a745;
        }

        .status-inactive {
          color: #dc3545;
        }

        .status-cancelled {
          color: #6c757d;
        }

        .loading, .error {
          text-align: center;
          padding: 40px;
          color: #666;
        }

        .error {
          color: #dc3545;
        }

        .statistics-actions {
          margin-top: 20px;
          text-align: center;
        }

        .refresh-btn {
          background: #007bff;
          color: white;
          border: none;
          padding: 10px 20px;
          border-radius: 5px;
          cursor: pointer;
          font-size: 14px;
        }

        .refresh-btn:hover {
          background: #0056b3;
        }
      `}</style>
    </div>
  );
};

export default Statistics;