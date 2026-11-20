import dotenv from "dotenv";

dotenv.config();

const configuredCorsOrigins = (process.env.CORS_ORIGIN ?? "http://localhost:3000")
  .split(",")
  .map((origin) => origin.trim())
  .filter(Boolean);

if (configuredCorsOrigins.includes("*")) {
  throw new Error(
    "CORS_ORIGIN cannot contain '*' because credentialed requests are enabled; configure explicit origins",
  );
}

export const env = {
  port: Number(process.env.PORT ?? 4000),
  databaseUrl:
    process.env.DATABASE_URL ?? "postgres://postgres:postgres@localhost:5432/stellargrant",
  adminAddresses: (process.env.ADMIN_ADDRESSES ?? "").split(",").filter(Boolean),
  metricsAllowedIps: (process.env.METRICS_ALLOWED_IPS ?? "").split(",").filter(Boolean),
  sendgridApiKey: process.env.SENDGRID_API_KEY ?? "",
  metricsBasicAuthUser: process.env.METRICS_BASIC_AUTH_USER ?? "",
  metricsBasicAuthPassword: process.env.METRICS_BASIC_AUTH_PASSWORD ?? "",
  nodeEnv: process.env.NODE_ENV ?? "development",
  contractId: process.env.CONTRACT_ID ?? "",
  rpcUrl: process.env.RPC_URL ?? "https://rpc-futurenet.stellar.org",
  networkPassphrase: process.env.NETWORK_PASSPHRASE ?? "Test SDF Network ; September 2015",
  redisUrl: process.env.REDIS_URL ?? "",
  pinataJwt: process.env.PINATA_JWT ?? "",
  ipfsGateway: process.env.PINATA_GATEWAY ?? "https://gateway.pinata.cloud",
  corsOrigins: configuredCorsOrigins,
  jwtSecret: process.env.JWT_SECRET ?? "",
  // #1229 — key used to encrypt sensitive columns at rest (falls back to JWT_SECRET).
  fieldEncryptionKey: process.env.FIELD_ENCRYPTION_KEY ?? process.env.JWT_SECRET ?? "",
  // #1240 — configurable log level for winston
  logLevel: process.env.LOG_LEVEL ?? "info",
  rateLimitAlertWindowMinutes: Number(process.env.RATE_LIMIT_ALERT_WINDOW_MINUTES ?? 5),
  rateLimitAlertThreshold: Number(process.env.RATE_LIMIT_ALERT_THRESHOLD ?? 100),
  rateLimitAlertCooldownMinutes: Number(process.env.RATE_LIMIT_ALERT_COOLDOWN_MINUTES ?? 15),
};
