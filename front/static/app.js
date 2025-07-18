// static/app.js

// Configuration - can be overridden by environment
const config = {
    apiBaseUrl: window.ENV?.API_BASE_URL || 'http://127.0.0.1:8080/api/v1',
    peachEntityId: window.ENV?.PEACH_ENTITY_ID || "8ac7a4c8961da56701961e61c57a0241"
};

const API_BASE_URL = config.apiBaseUrl; 

let currentUserId = localStorage.getItem('currentUserId') || null;
let currentSubscriptionId = localStorage.getItem('currentSubscriptionId') || null;
let currentSubscriptionPlan = localStorage.getItem('currentSubscriptionPlan') || null;
let currentSubscriptionPrice = localStorage.getItem('currentSubscriptionPrice') || null;
let currentSubscriptionStatus = localStorage.getItem('currentSubscriptionStatus') || null;

// Track timeouts to prevent memory leaks
let activeTimeouts = new Set();


// --- Utility Functions ---
function validateEmail(email) {
    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    return emailRegex.test(email);
}

function validateName(name) {
    return name && name.trim().length >= 2;
}

function sanitizeInput(input) {
    if (typeof input !== 'string') return input;
    return input.replace(/<script\b[^<]*(?:(?!<\/script>)<[^<]*)*<\/script>/gi, '');
}

function handleApiError(response, defaultMessage) {
    if (response.status === 400) {
        return 'Invalid input. Please check your data and try again.';
    } else if (response.status === 404) {
        return 'Resource not found. Please verify the information.';
    } else if (response.status >= 500) {
        return 'Server error. Please try again later.';
    }
    return defaultMessage;
}

// --- UI Update Functions ---
function showMessage(elementId, message, type) {
    const element = document.getElementById(elementId);
    element.textContent = message;
    element.className = `message ${type}`;
    element.style.display = 'block';
}

function hideMessage(elementId) {
    const element = document.getElementById(elementId);
    element.style.display = 'none';
}

function updateUserInfoUI() {
    const userInfoDiv = document.getElementById('currentUserInfo');
    const createSubscriptionBtn = document.getElementById('createSubscriptionBtn');
    
    if (currentUserId) {
        const currentUserIdEl = document.getElementById('currentUserId');
        const currentUserEmailEl = document.getElementById('currentUserEmail');
        const currentUserNameEl = document.getElementById('currentUserName');
        
        if (currentUserIdEl) currentUserIdEl.textContent = currentUserId;
        if (currentUserEmailEl) currentUserEmailEl.textContent = localStorage.getItem('currentUserEmail') || 'N/A';
        if (currentUserNameEl) currentUserNameEl.textContent = localStorage.getItem('currentUserName') || 'N/A';
        if (userInfoDiv) userInfoDiv.style.display = 'block';
        if (createSubscriptionBtn) createSubscriptionBtn.disabled = false;
    } else {
        if (userInfoDiv) userInfoDiv.style.display = 'none';
        if (createSubscriptionBtn) createSubscriptionBtn.disabled = true;
    }
}

function updateSubscriptionInfoUI() {
    const subInfoDiv = document.getElementById('currentSubscriptionInfo');
    const initiatePaymentBtn = document.getElementById('initiatePaymentBtn');
    const processVoucherBtn = document.getElementById('processVoucherBtn');

    if (currentSubscriptionId) {
        const currentSubscriptionIdEl = document.getElementById('currentSubscriptionId');
        const currentSubscriptionPlanEl = document.getElementById('currentSubscriptionPlan');
        const currentSubscriptionPriceEl = document.getElementById('currentSubscriptionPrice');
        const currentSubscriptionStatusEl = document.getElementById('currentSubscriptionStatus');
        
        if (currentSubscriptionIdEl) currentSubscriptionIdEl.textContent = currentSubscriptionId;
        if (currentSubscriptionPlanEl) currentSubscriptionPlanEl.textContent = currentSubscriptionPlan;
        if (currentSubscriptionPriceEl) currentSubscriptionPriceEl.textContent = currentSubscriptionPrice;
        if (currentSubscriptionStatusEl) currentSubscriptionStatusEl.textContent = currentSubscriptionStatus;
        if (subInfoDiv) subInfoDiv.style.display = 'block';

        if (currentSubscriptionStatus === 'Pending') {
            if (initiatePaymentBtn) initiatePaymentBtn.disabled = false;
            if (processVoucherBtn) processVoucherBtn.disabled = false;
        } else {
            if (initiatePaymentBtn) initiatePaymentBtn.disabled = true;
            if (processVoucherBtn) processVoucherBtn.disabled = true;
        }
    } else {
        if (subInfoDiv) subInfoDiv.style.display = 'none';
        if (initiatePaymentBtn) initiatePaymentBtn.disabled = true;
        if (processVoucherBtn) processVoucherBtn.disabled = true;
    }
}

// --- API Calls ---

async function registerUser() {
    hideMessage('registerMessage');
    const email = sanitizeInput(document.getElementById('registerEmail').value.trim());
    const name = sanitizeInput(document.getElementById('registerName').value.trim());

    // Validate inputs
    if (!email || !name) {
        showMessage('registerMessage', 'Please fill in both email and name.', 'error');
        return;
    }

    if (!validateEmail(email)) {
        showMessage('registerMessage', 'Please enter a valid email address.', 'error');
        return;
    }

    if (!validateName(name)) {
        showMessage('registerMessage', 'Name must be at least 2 characters long.', 'error');
        return;
    }

    try {
        const response = await fetch(`${API_BASE_URL}/users/register`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ email, name })
        });
        const data = await response.json();
        if (response.ok) {
            currentUserId = data.id;
            localStorage.setItem('currentUserId', currentUserId);
            localStorage.setItem('currentUserEmail', data.email);
            localStorage.setItem('currentUserName', data.name);
            showMessage('registerMessage', `User registered! ID: ${data.id}`, 'success');
            updateUserInfoUI();
        } else {
            const errorMessage = handleApiError(response, `Error: ${data}`);
            showMessage('registerMessage', errorMessage, 'error');
        }
    } catch (error) {
        console.error('Error registering user:', error);
        showMessage('registerMessage', 'Network error. Please check your connection and try again.', 'error');
    }
}

async function loginUser() {
    hideMessage('loginMessage');
    const email = sanitizeInput(document.getElementById('loginEmail').value.trim());

    if (!email) {
        showMessage('loginMessage', 'Please enter an email to login.', 'error');
        return;
    }

    if (!validateEmail(email)) {
        showMessage('loginMessage', 'Please enter a valid email address.', 'error');
        return;
    }

    try {
    

        const storedEmail = localStorage.getItem('currentUserEmail');
        const storedId = localStorage.getItem('currentUserId');
        const storedName = localStorage.getItem('currentUserName');

        if (storedEmail === email && storedId) {
            currentUserId = storedId;
            showMessage('loginMessage', `Logged in as ${email}.`, 'success');
            updateUserInfoUI();
        } else {
            showMessage('loginMessage', `No user found with email: ${email}. Please register first.`, 'error');
            currentUserId = null;
            localStorage.removeItem('currentUserId');
            localStorage.removeItem('currentUserEmail');
            localStorage.removeItem('currentUserName');
            updateUserInfoUI();
        }
    } catch (error) {
        console.error('Error logging in user:', error);
        showMessage('loginMessage', 'An error occurred during login.', 'error');
    }
}


async function createSubscription() {
    hideMessage('subscriptionMessage');
    if (!currentUserId) {
        showMessage('subscriptionMessage', 'Please register or log in a user first.', 'error');
        return;
    }

    const selectElement = document.getElementById('subscriptionPlan');
    const planName = selectElement.value;
    
    let price;
    try {
        const selectedOption = selectElement.options[selectElement.selectedIndex];
        const priceMatch = selectedOption.text.match(/\((R\d+)\)/);
        
        if (!priceMatch) {
            showMessage('subscriptionMessage', 'Could not extract price from selected plan.', 'error');
            return;
        }
        
        price = parseFloat(priceMatch[1].substring(1));
        
        if (isNaN(price) || price <= 0) {
            showMessage('subscriptionMessage', 'Invalid price detected.', 'error');
            return;
        }
    } catch (error) {
        console.error('Error extracting price:', error);
        showMessage('subscriptionMessage', 'Error processing subscription plan.', 'error');
        return;
    }

    const requestData = {
        user_id: currentUserId,
        plan_name: planName,
        price: price
    };

    try {
        const response = await fetch(`${API_BASE_URL}/subscriptions/create`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(requestData)
        });

        const contentType = response.headers.get('content-type');
        let data;

        if (contentType && contentType.includes('application/json')) {
            data = await response.json();
        } else {
            const text = await response.text();
            throw new Error(`Server responded with non-JSON: ${text}`);
        }

        if (response.ok) {
            currentSubscriptionId = data.id;
            currentSubscriptionPlan = data.plan_name;
            currentSubscriptionPrice = data.price;
            currentSubscriptionStatus = data.status;
            localStorage.setItem('currentSubscriptionId', currentSubscriptionId);
            localStorage.setItem('currentSubscriptionPlan', currentSubscriptionPlan);
            localStorage.setItem('currentSubscriptionPrice', currentSubscriptionPrice);
            localStorage.setItem('currentSubscriptionStatus', currentSubscriptionStatus);
            showMessage('subscriptionMessage', `Subscription created! ID: ${data.id}, Status: ${data.status}`, 'success');
            updateSubscriptionInfoUI();
        } else {
            console.error('Server rejected request:', data);
            const errorMessage = data.error || data.message || JSON.stringify(data);
            showMessage('subscriptionMessage', `Error: ${errorMessage}`, 'error');
        }
    } catch (error) {
        console.error('Error creating subscription:', error);
        showMessage('subscriptionMessage', `Network or server error: ${error.message}`, 'error');
    }
}




// static/app.js - relevant part of initiatePayment function

async function initiatePayment() {
    hideMessage('paymentInitiateMessage');
    hideMessage('peachPaymentMessage');
    
    if (!currentUserId || !currentSubscriptionId || currentSubscriptionStatus !== 'Pending') {
        showMessage('paymentInitiateMessage', 'Please create a pending subscription first.', 'error');
        return;
    }

    const amount = parseFloat(currentSubscriptionPrice);

    try {
        // Clear previous checkout if exists
        const container = document.getElementById('checkout-container');
        container.innerHTML = '';

        const response = await fetch(`${API_BASE_URL}/payments/initiate`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                user_id: currentUserId,
                subscription_id: currentSubscriptionId,
                amount: amount,
            })
        });
        
        const data = await response.json();

        if (!response.ok) {
            throw new Error(data.message || 'Failed to initiate payment');
        }

        const { checkoutId, merchantTransactionId } = data;
        if (!checkoutId || !merchantTransactionId) {
            throw new Error('Missing required payment data from server');
        }

        // Store payment data in multiple places for redundancy
        window.currentPayment = {
            checkoutId,
            merchantTransactionId,
            subscriptionId: currentSubscriptionId
        };
        localStorage.setItem('currentPayment', JSON.stringify(window.currentPayment));

        showMessage('paymentInitiateMessage', 'Loading payment form...', 'info');

        // Verify Checkout is available
        if (typeof Checkout === 'undefined') {
            throw new Error('Peach Payments checkout not loaded');
        }

        const entityId = config.peachEntityId;
        
        // Initialize checkout with proper error handling
        try {
            const checkout = Checkout.initiate({
                key: entityId,
                checkoutId: checkoutId,
                options: {
  customCSS: `
    html, body {
        height: 100% !important;
        margin: 0 !important;
        padding: 0 !important;
    }

    .peach-checkout {
        height: 100% !important;
        display: flex !important;
        flex-direction: column !important;
        justify-content: center !important;
    }

    iframe {
        height: 100% !important;
    }
  `
},
                events: {
                    options: {
                        theme: {
                            brand: {
                                primary: "black",
                            },
                            cards: {
                                background: "white",
                                backgroundHover: "red",
                            },
                        },
                    }, 
                    onCompleted: (event) => {
                        console.log("Payment Completed:", event);
                        const txnId = event.merchantTransactionId || 
                                     (window.currentPayment?.merchantTransactionId) || 
                                     merchantTransactionId;
                        
                        if (!txnId) {
                            showMessage('peachPaymentMessage', 'Payment completed but could not process transaction ID', 'error');
                            return;
                        }
                        
                        showMessage('peachPaymentMessage', 'Payment completed successfully!', 'success');
                        // window.location.href = `/payment-result.html?id=${txnId}`;
     
                // Add a small delay before redirect to ensure the message is seen
                const timeoutId = setTimeout(() => {
                    window.location.href = `/payment-result.html?id=${txnId}`;
                    activeTimeouts.delete(timeoutId);
                }, 1500);
                activeTimeouts.add(timeoutId);
                    },

                    
                    onCancelled: (event) => {
                        console.log("Payment Cancelled:", event);
                        showMessage('peachPaymentMessage', 'Payment cancelled by user', 'error');
                    },
                    onExpired: (event) => {
                        console.log("Payment Expired:", event);
                        showMessage('peachPaymentMessage', 'Payment session expired', 'error');
                    },
                    onBeforePayment: () => true,
                },
            });

            checkout.render("#checkout-container");
        } catch (initError) {
            console.error('Checkout initialization error:', initError);
            throw new Error('Failed to initialize payment form');
        }

    } catch (error) {
        console.error('Payment initiation error:', error);
        showMessage('paymentInitiateMessage', `Error: ${error.message}`, 'error');
    }
}// Add notification functions
async function checkNotifications() {
    if (!currentUserId) {
        showMessage('notificationMessage', 'Please log in to check notifications.', 'error');
        return;
    }

    try {
        const response = await fetch(`${API_BASE_URL}/notifications/user/${currentUserId}`, {
            method: 'GET',
            headers: { 'Content-Type': 'application/json' }
        });

        const data = await response.json();
        if (response.ok) {
            displayNotifications(data);
        } else {
            showMessage('notificationMessage', `Error: ${data.message || 'Failed to fetch notifications'}`, 'error');
        }
    } catch (error) {
        console.error('Error fetching notifications:', error);
        showMessage('notificationMessage', 'An error occurred while fetching notifications.', 'error');
    }
}

function displayNotifications(notifications) {
    const container = document.getElementById('notificationsList');
    
    if (!notifications || notifications.length === 0) {
        container.innerHTML = '<p>No notifications found.</p>';
        return;
    }

    // Clear container
    container.innerHTML = '';
    
    const notificationsDiv = document.createElement('div');
    notificationsDiv.className = 'notifications-list';
    
    notifications.forEach(notification => {
        const date = new Date(notification.created_at).toLocaleDateString();
        const acknowledgedClass = notification.acknowledged ? 'acknowledged' : 'unacknowledged';
        
        const notificationDiv = document.createElement('div');
        notificationDiv.className = `notification-item ${acknowledgedClass}`;
        
        const messageP = document.createElement('p');
        const strongEl = document.createElement('strong');
        strongEl.textContent = notification.message; // Safe text content
        messageP.appendChild(strongEl);
        
        const dateP = document.createElement('p');
        dateP.className = 'notification-date';
        dateP.textContent = `Date: ${date}`;
        
        const statusP = document.createElement('p');
        statusP.className = 'notification-status';
        statusP.textContent = `Status: ${notification.acknowledged ? 'Read' : 'Unread'}`;
        
        notificationDiv.appendChild(messageP);
        notificationDiv.appendChild(dateP);
        notificationDiv.appendChild(statusP);
        
        if (!notification.acknowledged) {
            const ackBtn = document.createElement('button');
            ackBtn.className = 'ack-btn';
            ackBtn.textContent = 'Mark as Read';
            ackBtn.onclick = () => acknowledgeNotification(notification.id);
            notificationDiv.appendChild(ackBtn);
        }
        
        notificationsDiv.appendChild(notificationDiv);
    });
    
    container.appendChild(notificationsDiv);
    showMessage('notificationMessage', `Found ${notifications.length} notifications`, 'success');
}

async function acknowledgeNotification(notificationId) {
    try {
        const response = await fetch(`${API_BASE_URL}/notifications/${notificationId}/acknowledge`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' }
        });

        if (response.ok) {
            showMessage('notificationMessage', 'Notification marked as read', 'success');
            checkNotifications(); // Refresh the list
        } else {
            const data = await response.json();
            showMessage('notificationMessage', `Error: ${data.message || 'Failed to acknowledge notification'}`, 'error');
        }
    } catch (error) {
        console.error('Error acknowledging notification:', error);
        showMessage('notificationMessage', 'An error occurred while updating notification.', 'error');
    }
}

// Add subscription renewal function
async function renewSubscription() {
    if (!currentSubscriptionId) {
        showMessage('renewalMessage', 'No active subscription found.', 'error');
        return;
    }

    try {
        const response = await fetch(`${API_BASE_URL}/subscriptions/${currentSubscriptionId}/renew`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' }
        });

        const data = await response.json();
        if (response.ok) {
            showMessage('renewalMessage', 'Subscription renewed successfully!', 'success');
            // Update local storage
            currentSubscriptionStatus = data.status;
            localStorage.setItem('currentSubscriptionStatus', currentSubscriptionStatus);
            updateSubscriptionInfoUI();
        } else {
            showMessage('renewalMessage', `Error: ${data.message || 'Failed to renew subscription'}`, 'error');
        }
    } catch (error) {
        console.error('Error renewing subscription:', error);
        showMessage('renewalMessage', 'An error occurred during renewal.', 'error');
    }
}

// Add function to check subscription status
async function checkSubscriptionStatus() {
    if (!currentSubscriptionId) {
        showMessage('subscriptionStatusMessage', 'No subscription selected.', 'error');
        return;
    }

    try {
        const response = await fetch(`${API_BASE_URL}/subscriptions/${currentSubscriptionId}`, {
            method: 'GET',
            headers: { 'Content-Type': 'application/json' }
        });

        const data = await response.json();
        if (response.ok) {
            // Update current subscription data
            currentSubscriptionStatus = data.status;
            currentSubscriptionPlan = data.plan_name;
            currentSubscriptionPrice = data.price;
            
            // Update localStorage
            localStorage.setItem('currentSubscriptionStatus', currentSubscriptionStatus);
            localStorage.setItem('currentSubscriptionPlan', currentSubscriptionPlan);
            localStorage.setItem('currentSubscriptionPrice', currentSubscriptionPrice);
            
            updateSubscriptionInfoUI();
            showMessage('subscriptionStatusMessage', `Subscription status updated: ${data.status}`, 'success');
        } else {
            showMessage('subscriptionStatusMessage', `Error: ${data.message || 'Failed to fetch subscription'}`, 'error');
        }
    } catch (error) {
        console.error('Error checking subscription status:', error);
        showMessage('subscriptionStatusMessage', 'An error occurred while checking subscription status.', 'error');
    }
}




// --- Cleanup Functions ---
function cleanup() {
    // Clear all active timeouts
    activeTimeouts.forEach(timeoutId => {
        clearTimeout(timeoutId);
    });
    activeTimeouts.clear();
}

// --- Initialize UI on Load ---
document.addEventListener('DOMContentLoaded', () => {
    updateUserInfoUI();
    updateSubscriptionInfoUI();

    // Clear messages initially
    hideMessage('registerMessage');
    hideMessage('loginMessage');
    hideMessage('subscriptionMessage');
    hideMessage('voucherMessage');
    hideMessage('paymentInitiateMessage');
    hideMessage('peachPaymentMessage');
});

// Cleanup on page unload
window.addEventListener('beforeunload', cleanup);