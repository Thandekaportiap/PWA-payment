// static/app.js

const API_BASE_URL = 'http://127.0.0.1:8080/api/v1'; 

let currentUserId = localStorage.getItem('currentUserId') || null;
let currentSubscriptionId = localStorage.getItem('currentSubscriptionId') || null;
let currentSubscriptionPlan = localStorage.getItem('currentSubscriptionPlan') || null;
let currentSubscriptionPrice = localStorage.getItem('currentSubscriptionPrice') || null;
let currentSubscriptionStatus = localStorage.getItem('currentSubscriptionStatus') || null;


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
    if (currentUserId) {
        document.getElementById('currentUserId').textContent = currentUserId;
        document.getElementById('currentUserEmail').textContent = localStorage.getItem('currentUserEmail') || 'N/A';
        document.getElementById('currentUserName').textContent = localStorage.getItem('currentUserName') || 'N/A';
        userInfoDiv.style.display = 'block';
        document.getElementById('createSubscriptionBtn').disabled = false;
    } else {
        userInfoDiv.style.display = 'none';
        document.getElementById('createSubscriptionBtn').disabled = true;
    }
}

function updateSubscriptionInfoUI() {
    const subInfoDiv = document.getElementById('currentSubscriptionInfo');
    const initiatePaymentBtn = document.getElementById('initiatePaymentBtn');
    const processVoucherBtn = document.getElementById('processVoucherBtn');

    if (currentSubscriptionId) {
        document.getElementById('currentSubscriptionId').textContent = currentSubscriptionId;
        document.getElementById('currentSubscriptionPlan').textContent = currentSubscriptionPlan;
        document.getElementById('currentSubscriptionPrice').textContent = currentSubscriptionPrice;
        document.getElementById('currentSubscriptionStatus').textContent = currentSubscriptionStatus;
        subInfoDiv.style.display = 'block';

        if (currentSubscriptionStatus === 'Pending') {
            initiatePaymentBtn.disabled = false;
            processVoucherBtn.disabled = false;
        } else {
            initiatePaymentBtn.disabled = true;
            processVoucherBtn.disabled = true;
        }
    } else {
        subInfoDiv.style.display = 'none';
        initiatePaymentBtn.disabled = true;
        processVoucherBtn.disabled = true;
    }
}

// --- API Calls ---

async function registerUser() {
    hideMessage('registerMessage');
    const email = document.getElementById('registerEmail').value;
    const name = document.getElementById('registerName').value;

    if (!email || !name) {
        showMessage('registerMessage', 'Please fill in both email and name.', 'error');
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
            showMessage('registerMessage', `Error: ${data}`, 'error');
        }
    } catch (error) {
        console.error('Error registering user:', error);
        showMessage('registerMessage', 'An error occurred during registration.', 'error');
    }
}

async function loginUser() {
    hideMessage('loginMessage');
    const email = document.getElementById('loginEmail').value;

    if (!email) {
        showMessage('loginMessage', 'Please enter an email to login.', 'error');
        return;
    }

    try {
        // In a real app, login would involve password. Here we simulate by finding user by email
        // and setting currentUserId. Backend doesn't have a login by email endpoint, so we simulate.
        // For testing purposes, we'll just check if the email exists from previously registered.
        // A proper login would return the user ID and confirm credentials.
        // For now, let's assume if the email exists, we can "log in".
        // This part needs a proper backend endpoint for "get user by email" or "login".

        // As a workaround for testing, we can register the user if they don't exist
        // or prompt them to register.
        // For now, let's just make sure a user is set as "current" if their ID is known.
        // The current backend doesn't support "get user by email".
        // So, for this frontend, logging in via email is symbolic.
        // We'll just assume `registerUser` is the primary way to get a `currentUserId`.

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
    const price = parseFloat(selectElement.options[selectElement.selectedIndex].text.match(/\((R\d+)\)/)[1].substring(1));

    try {
        const response = await fetch(`${API_BASE_URL}/subscriptions/create`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                user_id: currentUserId,
                plan_name: planName,
                price: price
            })
        });
        const data = await response.json();
        if (response.ok) {
            currentSubscriptionId = data.id;
            currentSubscriptionPlan = data.plan_name;
            currentSubscriptionPrice = data.price;
            currentSubscriptionStatus = data.status; // Should be "Pending"
            localStorage.setItem('currentSubscriptionId', currentSubscriptionId);
            localStorage.setItem('currentSubscriptionPlan', currentSubscriptionPlan);
            localStorage.setItem('currentSubscriptionPrice', currentSubscriptionPrice);
            localStorage.setItem('currentSubscriptionStatus', currentSubscriptionStatus);
            showMessage('subscriptionMessage', `Subscription created! ID: ${data.id}, Status: ${data.status}`, 'success');
            updateSubscriptionInfoUI();
        } else {
            showMessage('subscriptionMessage', `Error: ${JSON.stringify(data)}`, 'error');
        }
    } catch (error) {
        console.error('Error creating subscription:', error);
        showMessage('subscriptionMessage', 'An error occurred during subscription creation.', 'error');
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

        const entityId = "8ac7a4c8961da56701961e61c57a0241"; // Verify this is correct
        
        // Initialize checkout with proper error handling
        try {
            const checkout = Checkout.initiate({
                key: entityId,
                checkoutId: checkoutId,
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
                        window.location.href = `/payment-result.html?id=${txnId}`;
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