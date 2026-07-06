# SimpleFIN Authentication Integration

**Status**: Specification  
**Version**: 1.0  
**Last Updated**: 2026-07-06  

## Overview

This specification defines the authentication flow for integrating SimpleFIN, an aggregated financial data service, with the Momentum budgeting application. The flow enables users to securely connect their financial accounts via a setup token and store encrypted credentials in the OS keychain.

## 1. Authentication Flow

### 1.1 User Interaction Flow

```
User                        App                     SimpleFIN
 |                           |                           |
 +------Settings page------->|                           |
 |<----- SimpleFIN modal-----|                           |
 |                           |                           |
 +--paste setup token------->|                           |
 |                           +-------POST /claim------->|
 |                           |<-- access_url {auth}---+|
 |                           |                           |
 |                           +-- store in keychain      |
 |                           |                           |
 |                           +-- test GET /accounts---->|
 |                           |<---- {accounts}----------+|
 |<---- Success confirmation-|                           |
 |                           |                           |
```

### 1.2 Step-by-Step Process

1. **Access Setup**: User opens App Settings → Data Sources → Add SimpleFIN
2. **Get Setup Token**: User navigates to https://simplefin.com/ and generates a setup token (single-use)
3. **Paste Token**: User copies the token and pastes it into the app modal
4. **Claim Token**: App sends POST request to `https://auth.simplefin.com/claim` with setup token
5. **Receive Credentials**: SimpleFIN returns access URL with embedded basic auth credentials
6. **Secure Storage**: App stores access URL in OS keychain (never in database)
7. **Verify Connection**: App makes test GET request to verify credentials are valid
8. **Display Result**: User sees success confirmation with linked accounts

## 2. Claim Endpoint

### 2.1 Request Format

**Endpoint**: `POST https://auth.simplefin.com/claim`

**Headers**:
```
Content-Type: application/json
User-Agent: Momentum/1.0 (compatible with SimpleFIN API)
```

**Request Body**:
```json
{
  "setup_token": "SETUP_TOKEN_HERE"
}
```

**Example Request**:
```bash
curl -X POST https://auth.simplefin.com/claim \
  -H "Content-Type: application/json" \
  -d '{
    "setup_token": "https://simplefin.com/sync/setup/abc123def456"
  }'
```

### 2.2 Response Format

**Success Response (HTTP 200)**:
```json
{
  "access_url": "https://user:password@simplefin.com/api/v3/accounts"
}
```

**Example Success Response**:
```json
{
  "access_url": "https://alice_example:edc61d1fc9b6d5a8e7c4b3a29f8e1d6c@simplefin.com/api/v3/accounts"
}
```

### 2.3 Error Responses

**Invalid Token (HTTP 400)**:
```json
{
  "error": "invalid_token",
  "message": "The provided setup token is invalid or malformed"
}
```

**Token Expired/Already Used (HTTP 410)**:
```json
{
  "error": "token_claimed",
  "message": "This setup token has already been claimed or has expired"
}
```

**Server Error (HTTP 500)**:
```json
{
  "error": "server_error",
  "message": "An error occurred while processing your request"
}
```

### 2.4 Token Lifecycle

- **Setup Token**: Single-use, provided by SimpleFIN website, valid for 24 hours from generation
- **Access URL**: Long-lived, does not expire through timeout
- **Revocation**: User can revoke access at any time via SimpleFIN website dashboard
- **Reuse**: After claiming, setup token becomes invalid; user must generate a new token to reconnect

## 3. Secure Credential Storage

The access URL contains embedded credentials and must be stored securely in the OS keychain, never in the application database.

### 3.1 macOS - Keychain

**Storage Location**: User's default keychain  
**API**: SecKeychainItemCopyContent (Security framework)

**Implementation Details**:
```
Service Name: com.momentum.simplefin
Account Name: <setup_identifier>
Item Type: Generic Password
Data: access_url (full URL including credentials)
```

**Code Pattern**:
```
1. Create/update keychain item with SecKeychainAddGenericPassword() or SecKeychainItemModifyContent()
2. Retrieve with SecKeychainFindGenericPassword()
3. Always use kSecAttrAccessibleWhenUnlockedThisDeviceOnly for security
4. Handle user permission prompts gracefully
```

**Keychain Access Control**:
- Restrict access to the Momentum app only
- Require biometric/password authentication on first access per session
- Use `kSecAttrAccessibleWhenUnlockedThisDeviceOnly` attribute

### 3.2 Windows - Credential Manager

**Storage Location**: Windows Credential Manager (User Profile)  
**API**: CredWrite (Windows API), or use credential-storage crates

**Implementation Details**:
```
Target Name: Momentum:SimpleFIN:<setup_identifier>
Type: CRED_TYPE_GENERIC
Data: access_url (full URL including credentials)
Persistence: CRED_PERSIST_LOCAL_MACHINE or CRED_PERSIST_SESSION
```

**Code Pattern**:
```
1. Create CREDENTIAL structure with CredWrite()
2. Retrieve with CredRead()
3. Ensure secure buffer handling (SecureString or equivalent)
4. Clean up sensitive data after use
```

**Access Control**:
- Store as CRED_TYPE_GENERIC for enhanced security
- Use CRED_PERSIST_LOCAL_MACHINE for profile-level persistence
- Restrict to current user

### 3.3 Linux - secret-service or libsecret

**Storage Location**: Secret Service DBus daemon or libsecret backend  
**API**: libsecret (Vala-style API) or secret-service D-Bus

**Implementation Details**:
```
Collection: default (or "login")
Label: Momentum SimpleFIN Access
Attributes:
  - app: momentum
  - service: simplefin
  - identifier: <setup_identifier>
Secret: access_url (full URL including credentials)
```

**Code Pattern**:
```
1. Use secret_password_store() with identifier attributes
2. Retrieve with secret_password_lookup()
3. Implement fallback for systems without secret-service
4. Handle DBus connection errors gracefully
```

**Access Control**:
- Store in user's login keyring
- Mark as private/unlocked only during active session
- Require user authentication on first access

### 3.4 Cross-Platform Implementation Guidelines

- **No Fallback to Database**: Never store access URL in app database if keychain fails
- **Graceful Degradation**: If keychain unavailable, prompt user to re-authenticate rather than storing plaintext
- **Secure Clearing**: Clear sensitive data from memory immediately after use
- **Error Messages**: Log technical errors; show user-friendly messages to UI
- **Multi-Account**: Support multiple SimpleFIN accounts with unique identifiers

## 4. Connection Validation

### 4.1 Test Connection Flow

After receiving the access URL from SimpleFIN, the app must verify it works before marking setup as complete.

**Validation Request**:
```
GET https://user:password@simplefin.com/api/v3/accounts
Accept: application/json
```

**Success Response (HTTP 200)**:
```json
{
  "accounts": [
    {
      "id": "580a1f3979f0a7c4a8e29b7c",
      "name": "Checking Account",
      "currency": "USD",
      "balance": {
        "amount": 5342.18,
        "timestamp": 1625000000
      }
    }
  ]
}
```

**Validation Timeout**: 10 seconds (adjust based on network conditions)

### 4.2 Error Handling

| Status Code | Error Type | User Message | Recovery |
|---|---|---|---|
| 200 | Success | "SimpleFIN connected successfully" | Proceed to setup completion |
| 401 | Unauthorized | "Invalid credentials. Please check your setup token." | Prompt to re-enter token |
| 403 | Forbidden | "Access denied. Please re-authorize on SimpleFIN." | Redirect to SimpleFIN website |
| 404 | Not Found | "SimpleFIN API endpoint not found." | Contact support |
| 503 | Service Unavailable | "SimpleFIN is currently unavailable. Try again in a few minutes." | Retry with exponential backoff |
| Network Timeout | Connection Failed | "Network error connecting to SimpleFIN. Check your internet connection." | Retry with user confirmation |

### 4.3 Validation States

```
INITIAL
  ↓
CLAIMING_TOKEN (POST to /claim)
  ├─→ CLAIM_FAILED ──→ [Show error, return to token entry]
  └─→ CLAIM_SUCCESS ──→ STORING_CREDENTIAL
       ↓
    STORED_CREDENTIAL ──→ TESTING_CONNECTION
       ├─→ TEST_FAILED ──→ [Show error, keep stored URL, offer retry]
       └─→ TEST_SUCCESS ──→ CONNECTED
```

## 5. Credential Refresh & Recovery

### 5.1 Access URL Validity

- **Expiration**: Access URLs do not have built-in expiration
- **Revocation**: User can revoke access at any time via SimpleFIN dashboard
- **Status Check**: Perform test connection periodically (e.g., before data sync)

### 5.2 Handling 401 Responses

If a sync or account fetch returns HTTP 401 Unauthorized:

1. Log the error with timestamp
2. Mark SimpleFIN as "Needs Re-authentication"
3. On next app launch, prompt user with:
   - "Your SimpleFIN connection has expired or been revoked"
   - "Please generate a new setup token and re-connect"
4. Offer button to open SimpleFIN setup dialog
5. Clear the stored access URL only after successful re-authentication

### 5.3 Retry Strategy

**For Transient Failures** (503, timeouts):
```
Attempt 1: Immediate retry
Attempt 2: After 5 seconds
Attempt 3: After 30 seconds
After 3 failures: Surface error to user, mark for retry later
```

**For Auth Failures** (401, 403):
```
Do not retry automatically
Prompt user immediately
Suggest generating new setup token
```

### 5.4 Disconnection Flow

User can manually disconnect SimpleFIN:

```
1. Settings → Data Sources → SimpleFIN → [Disconnect]
2. Show confirmation: "This will remove your SimpleFIN access credentials"
3. Clear keychain entry for SimpleFIN
4. Remove sync schedule for SimpleFIN accounts
5. Optionally allow user to re-authenticate later
```

## 6. Security Considerations

### 6.1 Credential Handling

- **In Transit**: Use HTTPS only (enforced by SimpleFIN endpoints)
- **At Rest**: Store only in OS keychain, never in app database, config files, or logs
- **In Memory**: Use secure buffers; clear after use
- **In Logs**: Never log full access URL or credentials; only log token claim events with redaction

### 6.2 Setup Token Safety

- **Display**: Show token in masked format in UI (display only first and last 4 chars)
- **Copy/Paste**: Use system clipboard; clear clipboard after 30 seconds
- **Local Storage**: Never persist setup token after claiming

### 6.3 User Authentication

- **OS Keychain Access**: Inherit OS-level security (Touch ID, Windows Hello, etc.)
- **No Additional Password**: SimpleFIN credentials stored separately from app password
- **Session Management**: Consider requiring re-authentication after app restart or inactivity

### 6.4 API Security

- **HTTPS Only**: All SimpleFIN API calls must use HTTPS
- **User-Agent Header**: Identify app to SimpleFIN for analytics/rate-limiting
- **Rate Limiting**: Respect SimpleFIN API rate limits (typically 5 requests/second)
- **Network Security**: Validate SSL certificates; reject self-signed certificates

## 7. Implementation Checklist

### Backend/API Layer
- [ ] Implement SimpleFIN claim endpoint integration
- [ ] Handle all error codes from SimpleFIN
- [ ] Validate access URL format before storing
- [ ] Implement test connection to /accounts endpoint
- [ ] Add logging for auth events (without credential exposure)

### Keychain Integration
- [ ] macOS: Implement SecKeychainItemCopyContent API usage
- [ ] Windows: Implement CredWrite/CredRead or credential-storage crate
- [ ] Linux: Implement secret-service DBus or libsecret bindings
- [ ] Cross-platform: Abstract keychain interface for each platform

### UI/UX
- [ ] SimpleFIN setup modal with token input field
- [ ] Loading states during claim and validation
- [ ] Error messaging for each failure type
- [ ] Success confirmation with account summary
- [ ] Settings panel for SimpleFIN connection status
- [ ] Disconnect/re-authenticate options

### Testing
- [ ] Unit tests for claim request/response handling
- [ ] Integration tests with SimpleFIN sandbox
- [ ] Keychain storage/retrieval tests (per platform)
- [ ] Error recovery testing (invalid token, expired token, network failure)
- [ ] Multi-account scenarios (multiple SimpleFIN connections if supported)

### Documentation
- [ ] User guide for SimpleFIN setup
- [ ] Troubleshooting guide
- [ ] API documentation for internal SimpleFIN integration layer
- [ ] Keychain security policies per platform

## 8. Example Implementation Flow

### Claiming Setup Token (Pseudocode)

```typescript
async function claimSimpleFINToken(setupToken: string): Promise<string> {
  try {
    // 1. Send claim request
    const response = await fetch('https://auth.simplefin.com/claim', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'User-Agent': 'Momentum/1.0',
      },
      body: JSON.stringify({ setup_token: setupToken }),
      timeout: 10000,
    });

    if (!response.ok) {
      const error = await response.json();
      throw new SimpleFINError(error.error, error.message);
    }

    const data = await response.json();
    const accessUrl = data.access_url;

    // 2. Validate access URL format
    validateAccessUrl(accessUrl);

    // 3. Store in keychain
    await storeInKeychain('simplefin_access', accessUrl);

    // 4. Test connection
    await testSimpleFINConnection(accessUrl);

    return accessUrl;
  } catch (error) {
    throw new SimpleFINError('claim_failed', error.message);
  }
}

async function testSimpleFINConnection(accessUrl: string): Promise<void> {
  try {
    const response = await fetch(`${accessUrl}/accounts`, {
      method: 'GET',
      headers: { 'Accept': 'application/json' },
      timeout: 10000,
    });

    if (response.status === 401) {
      throw new SimpleFINError('invalid_credentials', 
        'Setup token is invalid or expired');
    }

    if (!response.ok) {
      throw new SimpleFINError('connection_failed', 
        `HTTP ${response.status}: ${response.statusText}`);
    }

    const data = await response.json();
    if (!data.accounts || !Array.isArray(data.accounts)) {
      throw new SimpleFINError('invalid_response', 
        'Unexpected API response format');
    }
  } catch (error) {
    throw new SimpleFINError('test_failed', error.message);
  }
}
```

### Storing in Keychain (Platform-Specific)

**macOS**:
```swift
func storeInKeychain(service: String, account: String, data: String) {
  let query = [
    kSecClass: kSecClassGenericPassword,
    kSecAttrService: service,
    kSecAttrAccount: account,
    kSecAttrAccessible: kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
    kSecValueData: data.data(using: .utf8)!
  ] as CFDictionary

  SecKeychainItemDelete(query as! SecKeychainItem)
  SecKeychainAddGenericPassword(...)
}
```

**Windows**:
```csharp
void StoreCredential(string service, string username, string accessUrl) {
  var credential = new NetworkCredential();
  credential.UserName = username;
  credential.Password = accessUrl;

  CredentialSet credSet = new CredentialSet(service);
  credSet.AddPassword(credential.UserName, credential.Password);
}
```

**Linux**:
```python
secret = Secret()
secret.store_sync(
  collection='default',
  label='Momentum SimpleFIN Access',
  attributes={
    'app': 'momentum',
    'service': 'simplefin'
  },
  secret=access_url,
  cancellable=None
)
```

## 9. References

- **SimpleFIN API Documentation**: https://simplefin.com/
- **SimpleFIN Setup Process**: https://simplefin.com/setup
- **macOS Keychain Security Framework**: https://developer.apple.com/documentation/security/keychain_services
- **Windows Credential Manager API**: https://docs.microsoft.com/en-us/windows/win32/api/wincred/
- **Linux secret-service specification**: https://standards.freedesktop.org/secret-service/

## 10. Revision History

| Version | Date | Changes |
|---|---|---|
| 1.0 | 2026-07-06 | Initial specification |
