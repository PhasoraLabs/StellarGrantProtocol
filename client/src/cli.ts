#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { Command } from "commander";
import dotenv from "dotenv";
import { Keypair, TransactionBuilder } from "@stellar/stellar-sdk";
import { StellarGrantsSDK } from "./StellarGrantsSDK";
import { WalletAdapter } from "./types";

type OutputFormat = "json" | "table";

type CliConfig = {
  contractId: string;
  rpcUrl: string;
  networkPassphrase: string;
};

class SecretKeySigner implements WalletAdapter {
  readonly name = "SecretKey";
  private readonly keypair: Keypair;

  constructor(secret: string) {
    this.keypair = Keypair.fromSecret(secret);
  }

  isAvailable(): boolean {
    return true;
  }

  async getPublicKey(): Promise<string> {
    return this.keypair.publicKey();
  }

  async signTransaction(txXdr: string, networkPassphrase: string): Promise<string> {
    const tx = TransactionBuilder.fromXDR(txXdr, networkPassphrase);
    tx.sign(this.keypair);
    return tx.toXDR();
  }
}

export function resolveCliConfig(overrides: Partial<CliConfig> = {}): CliConfig {
  const contractId = overrides.contractId ?? process.env.CONTRACT_ID;
  const rpcUrl = overrides.rpcUrl ?? process.env.RPC_URL;
  const networkPassphrase = overrides.networkPassphrase ?? process.env.NETWORK_PASSPHRASE;

  const missing: string[] = [];
  if (!contractId) missing.push("CONTRACT_ID");
  if (!rpcUrl) missing.push("RPC_URL");
  if (!networkPassphrase) missing.push("NETWORK_PASSPHRASE");

  if (missing.length > 0) {
    throw new Error(`Missing required connection credentials: ${missing.join(", ")}`);
  }

  return {
    contractId: contractId as string,
    rpcUrl: rpcUrl as string,
    networkPassphrase: networkPassphrase as string,
  };
}

function printOutput(data: Record<string, unknown>, format: OutputFormat): void {
  if (format === "json") {
    process.stdout.write(`${JSON.stringify(data, null, 2)}\n`);
    return;
  }
  console.table(data);
}

function writeEnvTemplate(targetPath: string, force = false): string {
  const resolved = path.resolve(targetPath);
  if (fs.existsSync(resolved) && !force) {
    throw new Error(`Refusing to overwrite existing file: ${resolved}. Use --force to overwrite.`);
  }

  const template = [
    "# StellarGrants SDK CLI config",
    "CONTRACT_ID=",
    "RPC_URL=https://soroban-testnet.stellar.org",
    "NETWORK_PASSPHRASE=Test SDF Network ; September 2015",
    "CLI_SIGNER_SECRET=",
    "",
  ].join("\n");

  fs.writeFileSync(resolved, template, "utf8");
  return resolved;
}

async function runGrantStatus(
  grantId: string,
  options: { format: OutputFormat; contractId?: string; rpcUrl?: string; networkPassphrase?: string },
): Promise<void> {
  const cfg = resolveCliConfig(options);
  const sdk = new StellarGrantsSDK(cfg);
  const grant = await sdk.grantGet(Number(grantId));

  printOutput(
    {
      command: "grant-status",
      grantId: Number(grantId),
      status: "ok",
      grant,
    },
    options.format,
  );
}

async function runFundGrant(
  grantId: string,
  options: {
    token: string;
    amount: string;
    format: OutputFormat;
    contractId?: string;
    rpcUrl?: string;
    networkPassphrase?: string;
    secret?: string;
  },
): Promise<void> {
  const cfg = resolveCliConfig(options);
  const secret = options.secret ?? process.env.CLI_SIGNER_SECRET;
  if (!secret) {
    throw new Error("Missing signing credential: CLI_SIGNER_SECRET (or --secret)");
  }

  const signer = new SecretKeySigner(secret);
  const sdk = new StellarGrantsSDK({ ...cfg, signer });

  const tx = (await sdk.grantFund({
    grantId: Number(grantId),
    token: options.token,
    amount: BigInt(options.amount),
  })) as any;

  printOutput(
    {
      command: "fund-grant",
      grantId: Number(grantId),
      token: options.token,
      amount: options.amount,
      status: tx.status,
      hash: tx.hash,
    },
    options.format,
  );
}

export async function runCli(argv = process.argv): Promise<void> {
  const program = new Command();
  program
    .name("sg")
    .description("StellarGrants developer CLI")
    .option("--env <path>", "Path to .env file", ".env");

  program
    .command("init")
    .description("Create a starter .env configuration")
    .option("--path <path>", "Output path for env file", ".env")
    .option("--force", "Overwrite existing file", false)
    .action((options: { path: string; force: boolean }) => {
      const resolved = writeEnvTemplate(options.path, options.force);
      process.stdout.write(`Created ${resolved}\n`);
    });

  program
    .command("grant-status")
    .description("Fetch on-chain grant status")
    .argument("<grantId>", "Grant numeric ID")
    .option("--contract-id <id>")
    .option("--rpc-url <url>")
    .option("--network-passphrase <passphrase>")
    .option("--format <format>", "json|table", "table")
    .action(async (grantId: string, options: any) => {
      await runGrantStatus(grantId, options);
    });

  program
    .command("fund-grant")
    .description("Fund an existing grant")
    .argument("<grantId>", "Grant numeric ID")
    .requiredOption("--token <address>", "Token contract address")
    .requiredOption("--amount <amount>", "Amount in base units")
    .option("--contract-id <id>")
    .option("--rpc-url <url>")
    .option("--network-passphrase <passphrase>")
    .option("--secret <secret>", "Stellar secret key used for signing")
    .option("--format <format>", "json|table", "table")
    .action(async (grantId: string, options: any) => {
      await runFundGrant(grantId, options);
    });

  const envPath = (() => {
    const idx = argv.findIndex((arg) => arg === "--env");
    if (idx >= 0 && argv[idx + 1]) return argv[idx + 1];
    return ".env";
  })();
  dotenv.config({ path: envPath });

  await program.parseAsync(argv);
}

if (require.main === module) {
  runCli().catch((error) => {
    const message = error instanceof Error ? error.message : String(error);
    process.stderr.write(`${message}\n`);
    process.exitCode = 1;
  });
}
