# SDK Compatibility Matrix

This document tracks compatibility between SDK versions and smart contract versions.

## Version Compatibility

| SDK Version | Contract Version | Compatible | Notes |
|-------------|------------------|------------|-------|
| 0.1.0 | 1 | ✅ Yes | Initial release |
| 0.2.0+ | 1 | ✅ Yes | Added IPFS, optimistic UI, compatibility checks |

## Contract Interface Version

The SDK defines `CONTRACT_INTERFACE_VERSION = 1` which must match the contract's interface version.

## Checking Compatibility

### Programmatic Check

```typescript
import { StellarGrantsSDK } from "@stellargrants/client-sdk";

const sdk = new StellarGrantsSDK({
  // ... config
});

const compatibility = await sdk.checkCompatibility();

if (!compatibility.compatible) {
  console.warn(`
    SDK Version: ${compatibility.sdkVersion}
    Contract Version: ${compatibility.contractVersion}
    Warning: ${compatibility.warning}
  `);
}
```

### Expected Responses

#### ✅ Compatible

```json
{
  "compatible": true,
  "sdkVersion": 1,
  "contractVersion": 1
}
```

#### ⚠️ Version Mismatch

```json
{
  "compatible": false,
  "sdkVersion": 1,
  "contractVersion": 2,
  "warning": "The contract version is newer than the SDK; upgrade the SDK."
}
```

#### ⚠️ Contract Version Unknown

```json
{
  "compatible": true,
  "sdkVersion": 1,
  "contractVersion": null,
  "warning": "Could not determine contract interface version; falling back to compatibility mode."
}
```

## Upgrade Paths

### SDK is Older than Contract

**Scenario:** Contract version 2, SDK version 1

**Action:** Upgrade the SDK

```bash
npm update @stellargrants/client-sdk
```

**Impact:** New contract features may not be available in older SDK versions.

### Contract is Older than SDK

**Scenario:** Contract version 1, SDK version 2

**Action:** 
- Option A: Upgrade the contract (recommended)
- Option B: Downgrade the SDK to match contract version

```bash
npm install @stellargrants/client-sdk@0.1.0
```

**Impact:** SDK features requiring newer contract methods will fail.

### Unknown Contract Version

**Scenario:** Contract doesn't expose version information

**Action:** SDK operates in compatibility mode

**Impact:** No version warnings, but runtime errors possible if methods don't exist.

## Breaking Changes

### SDK Version 0.1.0 → 0.2.0

**New Features (Non-Breaking):**
- IPFS integration helpers
- Optimistic UI utilities
- Compatibility checking
- Enhanced documentation

**No Breaking Changes:** Version 0.2.0 is fully backward compatible with 0.1.0.

## Feature Support Matrix

| Feature | SDK 0.1.0 | SDK 0.2.0+ | Contract v1 |
|---------|-----------|------------|-------------|
| Grant Creation | ✅ | ✅ | ✅ |
| Grant Funding | ✅ | ✅ | ✅ |
| Milestone Submit | ✅ | ✅ | ✅ |
| Milestone Vote | ✅ | ✅ | ✅ |
| IPFS Upload | ❌ | ✅ | ✅ |
| IPFS Fetch | ❌ | ✅ | ✅ |
| Optimistic UI | ❌ | ✅ | ✅ |
| Transaction Tracking | ✅ | ✅ | ✅ |
| Compatibility Check | ❌ | ✅ | ✅* |

\* Requires contract to implement `sdk_version()` method

## Testing Compatibility

### Manual Test

```typescript
import { StellarGrantsSDK } from "@stellargrants/client-sdk";

async function testCompatibility() {
  const sdk = new StellarGrantsSDK({
    contractId: "your_contract_id",
    rpcUrl: "your_rpc_url",
    networkPassphrase: "your_network",
    signer: yourSigner,
  });

  try {
    const compat = await sdk.checkCompatibility();
    
    if (compat.compatible) {
      console.log("✅ SDK and contract are compatible");
    } else {
      console.error("❌ Version mismatch:", compat.warning);
    }
  } catch (error) {
    console.error("Error checking compatibility:", error);
  }
}
```

### Automated Testing

Include compatibility checks in your test suite:

```typescript
describe("SDK Compatibility", () => {
  it("should be compatible with deployed contract", async () => {
    const sdk = new StellarGrantsSDK(config);
    const compat = await sdk.checkCompatibility();
    
    expect(compat.compatible).toBe(true);
  });
});
```

## Reporting Issues

If you encounter compatibility issues:

1. Check this document for known issues
2. Verify SDK and contract versions
3. Run `sdk.checkCompatibility()`
4. Report issues at: https://github.com/StellarGrant/StellarGrant-fe/issues

Include:
- SDK version
- Contract version (if available)
- Network (testnet/mainnet)
- Error messages
- Code snippet

## Future Versions

### Planned for SDK 0.3.0

- WebSocket event streaming
- Batch operations
- Multi-signature support
- Enhanced error recovery

### Contract v2 (Planned)

When contract v2 is released:
- SDK 0.3.0+ will support both v1 and v2
- Automatic version detection and adaptation
- Deprecation notices for v1-only features

## Best Practices

1. **Always check compatibility** on SDK initialization in production
2. **Pin SDK versions** in package.json for production apps
3. **Test thoroughly** after SDK or contract upgrades
4. **Monitor warnings** from compatibility checks
5. **Keep SDK updated** to match contract versions
6. **Use semantic versioning** to track compatibility

## Support

For compatibility questions:
- Documentation: [README.md](./README.md)
- API Reference: [API_REFERENCE.md](./API_REFERENCE.md)
- GitHub Issues: https://github.com/StellarGrant/StellarGrant-fe/issues
