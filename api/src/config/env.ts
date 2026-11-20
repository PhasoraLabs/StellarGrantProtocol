import dotenv from "dotenv";

dotenv.config();

export type RuntimeEnv = { port: number; databaseUrl: string };

export function parseEnv(input: NodeJS.ProcessEnv = process.env): RuntimeEnv {
  const port = Number(input.PORT ?? 4000);
  const databaseUrl = input.DATABASE_URL ?? "postgres://postgres:postgres@localhost:5432/stellargrant";
  if (!Number.isInteger(port) || port < 1 || port > 65535) throw new Error("PORT must be a valid TCP port");
  if (!databaseUrl.trim()) throw new Error("DATABASE_URL must not be empty");
  return { port, databaseUrl };
}

export const env = parseEnv();
