# Outbound Webhook System

This document describes the outbound webhook system for StellarGrant, enabling external systems and communities to receive real-time notifications for platform events.

## Overview

The webhook system provides:
- **Webhook subscriptions**: Users can register URLs for specific events
- **HMAC-SHA256 payload signing**: Ensures payload authenticity for receivers
- **Automatic retry logic**: Failed deliveries are retried with exponential backoff
- **Delivery logs**: Track every delivery attempt with status and response details
- **Auto-disable**: Subscriptions are automatically disabled after persistent failures

## Supported Events

| Event | Description |
|-------|-------------|
| `grant.created` | New grant created on the platform |
| `grant.updated` | Grant metadata updated |
| `grant.status_changed` | Grant status changed (e.g., approved, rejected, completed) |
| `milestone.submitted` | Milestone proof submitted |
| `milestone.approved` | Milestone approved by reviewer |
| `milestone.rejected` | Milestone rejected by reviewer |
| `contributor.blacklisted` | Contributor blacklisted |
| `contributor.reputation_changed` | Contributor reputation score changed |
| `fee.collected` | Platform fee collected |
| `community.created` | New community created |
| `community.updated` | Community metadata updated |
| `watchlist.added` | Grant added to watchlist |
| `watchlist.removed` | Grant removed from watchlist |
| `*` (wildcard) | Subscribe to all events |

## API Endpoints

### Register a Webhook
```
POST /webhooks
```
Body:
```json
{
  "targetUrl": "https://example.com/webhook",
  "secretKey": "your-webhook-secret-min-16-chars",
  "events": ["grant.created", "milestone.approved"],
  "communityId": 1
}
```

### List My Webhooks
```
GET /webhooks
```

### Get a Webhook
```
GET /webhooks/:id
```

### Update a Webhook
```
PATCH /webhooks/:id
```
Body:
```json
{
  "targetUrl": "https://new-url.com/webhook",
  "events": ["grant.created"],
  "isActive": true
}
```

### Delete a Webhook
```
DELETE /webhooks/:id
```

### Get Delivery Logs
```
GET /webhooks/:id/logs?page=1&limit=20
```

### Test a Webhook
```
POST /webhooks/test
```
Body:
```json
{
  "targetUrl": "https://example.com/webhook",
  "secretKey": "your-webhook-secret",
  "event": "grant.created"
}
```

## Payload Format

```json
{
  "event": "grant.created",
  "timestamp": "2024-01-15T10:30:00.000Z",
  "data": {
    "grantId": 123,
    "title": "Community Garden",
    "recipient": "G...",
    "totalAmount": 50000
  }
}
```

## Headers

Every webhook request includes these headers:

| Header | Description |
|--------|-------------|
| `Content-Type` | `application/json` |
| `X-Webhook-Signature` | `sha256=<hmac_hex>` |
| `X-Webhook-Event` | Event type (e.g., `grant.created`) |
| `X-Webhook-Id` | Delivery log ID for tracking |
| `X-Webhook-Timestamp` | ISO 8601 timestamp |
| `User-Agent` | `StellarGrant-Webhook/1.0` |

## Verifying Signatures

To verify webhook authenticity, compute HMAC-SHA256 over the JSON payload body:

```typescript
import { createHmac } from "node:crypto";

function verifyWebhook(body: string, signature: string, secret: string): boolean {
  const sig = signature.replace("sha256=", "");
  const expected = createHmac("sha256", secret).update(body).digest("hex");
  return sig === expected;
}
```

## Retry Policy

| Attempt | Delay |
|---------|-------|
| 1st | 5 seconds |
| 2nd | 15 seconds |
| 3rd | 1 minute |
| 4th | 5 minutes |
| 5th | 15 minutes |

- Subscriptions are auto-disabled after 10 consecutive failures
- Each delivery has a 30-second timeout
- Retries are processed by a background queue every 30 seconds

## Integration Points

Webhooks are dispatched from these system events:

- **Grant sync service**: `grant.created`, `grant.status_changed`, `contributor.reputation_changed`
- **Milestone proof route**: `milestone.submitted`
- **Milestone approval route**: `milestone.approved`, `milestone.rejected`
- **Community routes**: `community.created`, `community.updated`

## Database Schema

### `webhook_subscriptions`
| Column | Type | Description |
|--------|------|-------------|
| `id` | int PK | Subscription ID |
| `target_url` | varchar | Destination URL |
| `secret_key` | varchar | HMAC secret |
| `events` | simple-array | List of subscribed events |
| `is_active` | boolean | Active/disabled state |
| `failure_count` | int | Consecutive failures |
| `max_retries` | int | Max retry attempts (default 5) |
| `community_id` | int | Optional community scope |
| `owner_address` | varchar | Stellar address of owner |
| `created_by` | int FK | User ID |

### `webhook_delivery_logs`
| Column | Type | Description |
|--------|------|-------------|
| `id` | int PK | Log ID |
| `subscription_id` | int FK | Related subscription |
| `event_type` | varchar | Event that triggered delivery |
| `payload` | jsonb | Full event payload |
| `payload_signature` | varchar | HMAC signature used |
| `status` | enum | `pending`, `delivered`, `failed`, `retrying`, `exhausted` |
| `attempt_count` | int | Number of delivery attempts |
| `http_status_code` | int | Last HTTP response code |
| `response_body` | text | Last response body |
| `error_message` | text | Error details |
| `next_retry_at` | timestamp | Scheduled retry time |
