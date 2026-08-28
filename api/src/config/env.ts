import dotenv from "dotenv";

dotenv.config();

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
};
