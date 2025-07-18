# Recurring Payments Implementation Guide

This guide explains the recurring payment functionality implemented for your subscription service using Peach Payments.

## Overview

The recurring payment system allows customers to:
1. Make an initial payment using Card, EFT, 1Voucher, or Scan to Pay
2. Automatically save their payment method details after successful payment
3. Use stored payment methods for future subscription renewals
4. Manage their saved payment methods

## How It Works

### 1. Initial Payment Flow
- Customer selects payment method (Card, EFT, Scan to Pay, or 1Voucher)
- Payment is processed through Peach Payments
- Upon successful payment, the system automatically extracts and stores payment method details
- Payment method is saved with a registration token from Peach for recurring use

### 2. Payment Method Storage
After a successful payment, the system automatically:
- Extracts payment method details from the Peach transaction
- Stores card details (last 4 digits, brand, expiry) or bank details for EFT
- Saves the Peach registration ID for recurring payments
- Sets the payment method as default for the user

### 3. Recurring Payment Process
For future payments, customers can:
- View their saved payment methods
- Select a saved method to pay with one click
- Process recurring payments using stored Peach registration tokens
- Remove unwanted payment methods

## API Endpoints

### Backend Endpoints

#### Get User Payment Methods
```
GET /api/v1/payments/payment-methods/{user_id}
```
Returns all active payment methods for a user.

#### Store Payment Method
```
POST /api/v1/payments/payment-methods/store
Content-Type: application/json

{
  "payment_id": "uuid",
  "set_as_default": true
}
```
Manually store a payment method from a completed payment.

#### Create Recurring Payment
```
POST /api/v1/payments/recurring
Content-Type: application/json

{
  "user_id": "uuid",
  "subscription_id": "uuid", 
  "amount": 100.0,
  "payment_method_detail_id": "uuid"
}
```
Process a recurring payment using a stored payment method.

#### Deactivate Payment Method
```
DELETE /api/v1/payments/payment-methods/{user_id}/{payment_method_id}
```
Remove a stored payment method.

### Frontend API Functions

```javascript
// Get user's saved payment methods
const paymentMethods = await getUserPaymentMethods(userId);

// Store a payment method from completed payment
await storePaymentMethod({
  payment_id: paymentId,
  set_as_default: true
});

// Process recurring payment
await createRecurringPayment({
  user_id: userId,
  subscription_id: subscriptionId,
  amount: 100.0,
  payment_method_detail_id: methodId
});

// Remove payment method
await deactivatePaymentMethod(userId, methodId);
```

## Database Schema

### PaymentMethodDetail Table
```rust
pub struct PaymentMethodDetail {
    pub id: Uuid,
    pub user_id: Uuid,
    pub payment_method: PaymentMethod, // Card, EFT, ScanToPay, Voucher
    pub peach_registration_id: Option<String>, // For recurring payments
    pub card_last_four: Option<String>,
    pub card_brand: Option<String>, // Visa, Mastercard, etc.
    pub expiry_month: Option<u8>,
    pub expiry_year: Option<u16>,
    pub bank_name: Option<String>, // For EFT
    pub is_default: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### Enhanced Payment Table
```rust
pub struct Payment {
    // ... existing fields ...
    pub peach_payment_id: Option<String>, // ID from Peach after successful payment
    pub is_recurring: bool,
    pub parent_payment_id: Option<Uuid>, // For recurring payments
}
```

## Supported Payment Methods

### 1. Credit/Debit Cards
- Supports Visa, Mastercard, American Express
- Stores card brand, last 4 digits, expiry date
- Supports recurring payments via Peach registration tokens

### 2. Instant EFT
- Supports major South African banks
- Stores bank name
- Supports recurring payments for certain banks

### 3. Scan to Pay
- QR code-based mobile payments
- Supported by major SA banks
- Instant payment confirmation

### 4. 1Voucher
- Prepaid voucher payments
- 16-digit PIN entry
- Does not support recurring payments (one-time use)

## Security Considerations

### Data Storage
- Full card details are NEVER stored locally
- Only necessary information for recurring payments is saved
- Card numbers are masked (only last 4 digits shown)
- Peach registration tokens are used for recurring transactions

### PCI Compliance
- The application follows PCI-DSS guidelines
- Sensitive card data flows through Peach Payments only
- No direct handling of full card numbers

### Access Control
- Users can only access their own payment methods
- All API endpoints validate user ownership
- Payment methods can be deactivated but not deleted (audit trail)

## Peach Payments Integration

### Registration Process
When a payment succeeds, the system:
1. Calls Peach's payment details API
2. Extracts the registration ID from the response
3. Stores the registration token for future use

### Recurring Payment Process
For recurring payments:
1. Uses stored registration ID
2. Calls Peach's recurring payment API
3. Processes payment without customer interaction
4. Updates subscription status on success

### Webhook Handling
The payment webhook automatically:
- Updates payment status
- Extracts payment method details
- Stores payment methods for recurring use
- Activates subscriptions on successful payment

## Frontend Components

### PaymentMethodManager
Displays and manages saved payment methods:
- Shows payment method details (masked card numbers, bank names)
- Enables one-click recurring payments
- Allows removal of payment methods
- Responsive design for mobile devices

### Enhanced PaymentForm
Updated to support all payment methods:
- Card payments via Peach Checkout
- EFT processing
- Scan to Pay QR codes
- 1Voucher PIN entry

## Configuration

### Environment Variables
Add these to your `.env` file:

```bash
# Peach Payments V1 (for payment status and details)
PEACH_V1_BASE_URL=https://eu-test.oppwa.com
PEACH_ENTITY_ID=your_entity_id
PEACH_ACCESS_TOKEN=your_access_token
PEACH_SECRET_KEY=your_secret_key

# Peach Payments V2 (for checkout and recurring)
PEACH_AUTH_SERVICE_URL=https://api.peachpayments.com/auth/token
PEACH_CHECKOUT_V2_ENDPOINT=https://api.peachpayments.com/checkout
PEACH_ENTITY_ID_V2=your_v2_entity_id
PEACH_CLIENT_ID=your_client_id
PEACH_CLIENT_SECRET=your_client_secret
PEACH_MERCHANT_ID=your_merchant_id

# Webhook URLs
PEACH_NOTIFICATION_URL=https://yourdomain.com/api/v1/payments/callback
PEACH_SHOPPER_RESULT_URL=https://yourdomain.com/payment-result.html
```

## Testing

### Test Cards
Use Peach Payments test card numbers:
- `4111111111111111` (Visa)
- `5555555555554444` (Mastercard)
- Any future expiry date
- Any 3-digit CVV

### Test Flow
1. Create a user account
2. Make an initial payment with a test card
3. Verify payment method is auto-stored
4. Use the Payment Method Manager to make a recurring payment
5. Verify the recurring payment processes successfully

## Error Handling

### Common Scenarios
- **Payment method not found**: Returns 404 with descriptive message
- **Inactive payment method**: Returns 400 with status explanation
- **No registration ID**: Returns 400 for non-recurring capable methods
- **Peach API errors**: Returns 500 with Peach error details

### Frontend Error Display
- Clear error messages for users
- Specific guidance for different error types
- Retry mechanisms for temporary failures

## Future Enhancements

### Planned Features
1. **Automatic Subscription Renewals**: Schedule recurring payments
2. **Payment Method Expiry Handling**: Notify users of expiring cards
3. **Multiple Default Methods**: Different defaults per payment type
4. **Payment History**: Full transaction history per payment method
5. **Refund Support**: Process refunds through stored payment methods

### Integration Opportunities
1. **Email Notifications**: Payment confirmations and failures
2. **SMS Notifications**: Payment reminders and confirmations
3. **Calendar Integration**: Payment due date reminders
4. **Analytics**: Payment method usage statistics

## Support and Troubleshooting

### Common Issues
1. **Registration ID Missing**: Some payment methods may not support recurring
2. **Webhook Delays**: Payment method storage may take a few seconds
3. **Bank Limitations**: Not all banks support recurring EFT payments

### Debug Information
- All API calls include detailed logging
- Payment webhook includes comprehensive status tracking
- Frontend displays clear error messages and status updates

---

This implementation provides a complete recurring payment solution that enhances user experience while maintaining security and compliance with payment industry standards.