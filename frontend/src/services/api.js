const API_BASE_URL = 'http://localhost:8080/api/v1'; // Fixed: Changed from 3001 to 8080

class ApiError extends Error {
  constructor(message, status) {
    super(message);
    this.status = status;
  }
}

const handleResponse = async (response) => {
  if (!response.ok) {
    let errorData = {};
    try {
      errorData = await response.json();
    } catch (e) {
      // If response is not JSON, use status text
      throw new ApiError(`HTTP ${response.status}: ${response.statusText}`, response.status);
    }

    // Try to find a specific error message, otherwise fallback or stringify the object
    const errorMessage = errorData.error || errorData.message || JSON.stringify(errorData) || `HTTP ${response.status}: ${response.statusText}`;
    throw new ApiError(errorMessage, response.status);
  }
  return response.json();
};

// --- Helper Functions ---
export const formatCurrency = (amount, currency = 'ZAR') => {
  return new Intl.NumberFormat('en-ZA', {
    style: 'currency',
    currency: currency,
  }).format(amount);
};

export const formatDate = (isoString) => {
  if (!isoString) return 'N/A';
  try {
    // Attempt to parse as NaiveDateTime string (YYYY-MM-DDTHH:MM:SS)
    // Add 'Z' to make it UTC if it's not already, for consistent parsing
    const date = new Date(isoString.endsWith('Z') ? isoString : `${isoString}Z`);
    if (isNaN(date.getTime())) {
      // Fallback for other formats if necessary
      return new Date(isoString).toLocaleString();
    }
    return date.toLocaleString();
  } catch (e) {
    console.error("Error parsing date:", isoString, e);
    return isoString; // Return original if parsing fails
  }
};

// --- User API ---
export const createUser = async (userData) => {
  const response = await fetch(`${API_BASE_URL}/users`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(userData),
  });
  return handleResponse(response);
};

export const getUser = async (userId) => {
  const response = await fetch(`${API_BASE_URL}/users/${userId}`);
  return handleResponse(response);
};

export const listUsers = async () => {
  const response = await fetch(`${API_BASE_URL}/users`);
  return handleResponse(response);
};

// --- Subscription API ---
export const createSubscription = async (subscriptionData) => {
  const response = await fetch(`${API_BASE_URL}/subscriptions`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(subscriptionData),
  });
  return handleResponse(response);
};

// Get subscription by user_id
export const getSubscription = async (userId) => {
  const response = await fetch(`${API_BASE_URL}/subscriptions/${userId}`);
  return handleResponse(response);
};

// Note: listSubscriptions and updateSubscription are not fully implemented in backend examples
// but kept here as placeholders if you expand your API.
export const listSubscriptions = async () => {
  const response = await fetch(`${API_BASE_URL}/subscriptions`);
  return handleResponse(response);
};

export const updateSubscription = async (subscriptionId, updateData) => {
  const response = await fetch(`${API_BASE_URL}/subscriptions/${subscriptionId}`, {
    method: 'PUT',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(updateData),
  });
  return handleResponse(response);
};


// --- Payment API ---
export const initiatePayment = async (paymentData) => {
  const response = await fetch(`${API_BASE_URL}/payment/initiate`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(paymentData),
  });
  return handleResponse(response);
};

export const getPaymentStatus = async (checkRequestData) => {
    // This expects { payment_id, peach_checkout_id, merchant_transaction_id } as per your Rust backend
    const response = await fetch(`${API_BASE_URL}/payment/status`, {
      method: 'POST', // Backend expects POST
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(checkRequestData),
    });
    return handleResponse(response);
};

// Note: listPayments is not fully implemented in backend examples
export const listPayments = async (filters = {}) => {
  // In a real app, you'd construct query params from filters
  const response = await fetch(`${API_BASE_URL}/payments`);
  return handleResponse(response);
};

// Modified pollPaymentStatus to pass correct parameters matching backend's CheckPaymentStatusRequest
export const pollPaymentStatus = (internalPaymentId, peachCheckoutId, merchantTransactionId, interval = 3000, maxAttempts = 10) => {
  return new Promise((resolve, reject) => {
    let attempts = 0;
    const poll = async () => {
      try {
        attempts++;
        console.log(`🔄 Polling payment status (attempt ${attempts}/${maxAttempts})`);
        
        // Pass all three required IDs with correct key names to getPaymentStatus
        const status = await getPaymentStatus({ 
            payment_id: internalPaymentId, 
            peach_checkout_id: peachCheckoutId, 
            merchant_transaction_id: merchantTransactionId 
        });
        
        // Check if payment is in a final state
        if (status.status === 'Completed' || status.status === 'Failed' || status.status === 'Canceled') { // Use capitalized statuses from Rust enum
          console.log(`✅ Payment final status: ${status.status}`);
          resolve(status);
          return;
        }
        
        // Continue polling if not in final state and haven't exceeded max attempts
        if (attempts < maxAttempts) {
          setTimeout(poll, interval);
        } else {
          console.log('⏰ Payment polling timeout');
          resolve(status); // Return last known status
        }
      } catch (error) {
        console.error('❌ Error polling payment status:', error);
        if (attempts < maxAttempts) {
          setTimeout(poll, interval);
        } else {
          reject(error);
        }
      }
    };
    
    poll();
  });
};

export const processVoucher = async (voucherData) => {
    const response = await fetch(`${API_BASE_URL}/subscriptions/voucher`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify(voucherData),
    });
    return handleResponse(response);
};

// --- Health Check ---
export const healthCheck = async () => {
  const response = await fetch(`${API_BASE_URL}/health`);
  if (!response.ok) {
    throw new Error('API is not healthy');
  }
  return response.json();
};

export default {
  // User methods
  createUser,
  getUser,
  listUsers,
  
  // Subscription methods
  createSubscription,
  getSubscription,
  listSubscriptions,
  updateSubscription,
  processVoucher,
  
  // Payment methods
  initiatePayment,
  getPaymentStatus,
  listPayments,
  pollPaymentStatus,

  // Utilities
  healthCheck,
  formatCurrency,
  formatDate,
};
