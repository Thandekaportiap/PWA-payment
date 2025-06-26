// frontend/src/components/VoucherInput.jsx
import React from 'react';

const VoucherInput = ({ value, onChange, disabled = false }) => {
  const handleInputChange = (e) => {
    // Only allow digits for a PIN, limit to 16 characters
    const numericValue = e.target.value.replace(/\D/g, '');
    if (numericValue.length <= 16) {
      onChange(numericValue);
    }
  };

  return (
    <div className="voucher-input-container">
      <label htmlFor="voucherPin" className="voucher-label">1Voucher PIN:</label>
      <input
        id="voucherPin"
        type="text"
        value={value}
        onChange={handleInputChange}
        placeholder="Enter 16-digit 1Voucher PIN"
        disabled={disabled}
        className="voucher-pin-input"
        maxLength={16} // Enforce max length
        inputMode="numeric" // Suggest numeric keyboard on mobile devices
        pattern="\d{16}" // HTML5 validation for exactly 16 digits
        title="Please enter a 16-digit 1Voucher PIN"
      />
      <p className="voucher-help-text">Enter the 16-digit PIN found on your 1Voucher slip.</p>
    </div>
  );
};

export default VoucherInput;