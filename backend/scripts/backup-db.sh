#!/usr/bin/env bash
# Database backup script for StellarGrant API.
# Dumps PostgreSQL, compresses, and uploads to S3.
# Retention: deletes backups older than 30 days from the bucket.
#
# Required env vars:
#   DATABASE_URL   - postgres connection string
#   S3_BUCKET      - target bucket name (e.g. my-company-db-backups)
#   AWS_REGION     - AWS region (default: us-east-1)
#
# Optional env vars:
#   RETENTION_DAYS - how many days to keep (default: 30)
#   S3_PREFIX      - key prefix inside the bucket (default: stellargrant/)

set -euo pipefail

: "${DATABASE_URL:?DATABASE_URL is required}"
: "${S3_BUCKET:?S3_BUCKET is required}"
AWS_REGION="${AWS_REGION:-us-east-1}"
RETENTION_DAYS="${RETENTION_DAYS:-30}"
S3_PREFIX="${S3_PREFIX:-stellargrant/}"

TIMESTAMP=$(date -u +"%Y%m%dT%H%M%SZ")
DUMP_FILE="/tmp/stellargrant-${TIMESTAMP}.dump"

echo "[backup] Starting database dump at ${TIMESTAMP}"
pg_dump --format=custom --no-acl --no-owner "${DATABASE_URL}" > "${DUMP_FILE}"
echo "[backup] Dump complete: ${DUMP_FILE}"

S3_KEY="${S3_PREFIX}${TIMESTAMP}.dump"
echo "[backup] Uploading to s3://${S3_BUCKET}/${S3_KEY}"
aws s3 cp "${DUMP_FILE}" "s3://${S3_BUCKET}/${S3_KEY}" \
  --region "${AWS_REGION}" \
  --sse AES256

rm -f "${DUMP_FILE}"
echo "[backup] Upload complete, local file removed"

# Rotate: delete objects older than RETENTION_DAYS
CUTOFF=$(date -u -d "${RETENTION_DAYS} days ago" +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null \
         || date -u -v -"${RETENTION_DAYS}"d +"%Y-%m-%dT%H:%M:%SZ")

echo "[backup] Pruning objects older than ${RETENTION_DAYS} days (before ${CUTOFF})"
aws s3api list-objects-v2 \
  --bucket "${S3_BUCKET}" \
  --prefix "${S3_PREFIX}" \
  --query "Contents[?LastModified<='${CUTOFF}'].Key" \
  --output text \
  --region "${AWS_REGION}" \
| tr '\t' '\n' \
| grep -v '^$' \
| while IFS= read -r key; do
    echo "[backup] Deleting old backup: ${key}"
    aws s3 rm "s3://${S3_BUCKET}/${key}" --region "${AWS_REGION}"
  done

echo "[backup] Done"
