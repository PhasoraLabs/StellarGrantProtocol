/**
 * Environment Variable Validation for Next.js
 * 
 * Validates required environment variables at startup.
 * Provides clear error messages in development and graceful degradation in production.
 */

interface EnvVarConfig {
  name: string;
  required: boolean;
  defaultValue?: string;
  description: string;
}

const ENV_VARS: EnvVarConfig[] = [
  {
    name: "NEXT_PUBLIC_STELLAR_RPC_URL",
    required: true,
    description: "Stellar Soroban RPC server URL",
  },
  {
    name: "NEXT_PUBLIC_NETWORK_PASSPHRASE",
    required: true,
    defaultValue: "Test SDF Network ; September 2015",
    description: "Stellar network passphrase",
  },
  {
    name: "NEXT_PUBLIC_CONTRACT_ID",
    required: true,
    description: "StellarGrants smart contract ID",
  },
];

// Name fragments (split on "_") that suggest a NEXT_PUBLIC_* variable is
// actually holding a credential rather than a value that's safe to ship to
// the browser. NEXT_PUBLIC_* variables are inlined into client-side JS at
// build time and are visible to anyone who loads the site.
const SECRET_NAME_FRAGMENTS = new Set(["JWT", "SECRET", "KEY", "TOKEN", "PASSWORD"]);

function looksLikeSecretName(varName: string): boolean {
  return varName
    .split("_")
    .some((fragment) => SECRET_NAME_FRAGMENTS.has(fragment.toUpperCase()));
}

interface ValidationResult {
  valid: boolean;
  errors: string[];
  warnings: string[];
  securityWarnings: string[];
}

export function validateEnv(): ValidationResult {
  const errors: string[] = [];
  const warnings: string[] = [];
  const securityWarnings: string[] = [];
  const isDevelopment = process.env.NODE_ENV !== "production";

  for (const key of Object.keys(process.env)) {
    if (key.startsWith("NEXT_PUBLIC_") && looksLikeSecretName(key)) {
      securityWarnings.push(
        `Possible credential exposed to the browser: ${key}\n` +
        `  NEXT_PUBLIC_* variables are bundled into client-side JavaScript and are ` +
        `publicly visible to anyone who loads the site.\n` +
        `  If ${key} holds a real secret, drop the NEXT_PUBLIC_ prefix and read it ` +
        `only from server-side code.`
      );
    }
  }

  for (const config of ENV_VARS) {
    const value = process.env[config.name];

    if (!value) {
      if (config.required) {
        errors.push(
          `Missing required environment variable: ${config.name}\n` +
          `  Description: ${config.description}\n` +
          `  Please set ${config.name} in your .env.local file.`
        );
      } else if (config.defaultValue) {
        if (isDevelopment) {
          warnings.push(
            `Using default value for ${config.name}: ${config.defaultValue}\n` +
            `  Description: ${config.description}\n` +
            `  Consider setting ${config.name} explicitly in your .env.local file.`
          );
        }
      }
    } else if (value.trim() === "" && config.required) {
      errors.push(
        `Empty value for required environment variable: ${config.name}\n` +
        `  Description: ${config.description}\n` +
        `  Please provide a non-empty value for ${config.name}.`
      );
    }
  }

  return {
    valid: errors.length === 0,
    errors,
    warnings,
    securityWarnings,
  };
}

export function logValidationResult(result: ValidationResult): void {
  const isDevelopment = process.env.NODE_ENV !== "production";

  // Security warnings are logged regardless of environment: a leaked
  // credential is most dangerous in production, which is exactly where
  // the regular dev-only warnings below would otherwise hide it.
  if (result.securityWarnings.length > 0) {
    console.error("\n🔒 Environment Variable Security Warnings:");
    result.securityWarnings.forEach((warning) => {
      console.error(`  ${warning}`);
    });
    console.error("");
  }

  if (result.warnings.length > 0 && isDevelopment) {
    console.warn("\n⚠️  Environment Variable Warnings:");
    result.warnings.forEach((warning) => {
      console.warn(`  ${warning}`);
    });
    console.warn("");
  }

  if (result.errors.length > 0) {
    console.error("\n❌ Environment Variable Validation Failed:");
    result.errors.forEach((error) => {
      console.error(`  ${error}`);
    });
    console.error("");
  } else if (isDevelopment) {
    console.log("✅ Environment variables validated successfully");
  }
}

export function validateEnvOnStartup(): ValidationResult {
  const result = validateEnv();
  logValidationResult(result);

  if (!result.valid) {
    const isDevelopment = process.env.NODE_ENV !== "production";
    
    if (isDevelopment) {
      // In development, we want to show the error prominently
      console.error(
        "⚠️  Application may not function correctly due to missing environment variables."
      );
    } else {
      // In production, log error but continue with degraded mode
      console.error(
        "⚠️  Running in degraded mode due to missing environment variables."
      );
      console.error(
        "Some features may not work correctly. Please check the logs above."
      );
    }
  }

  return result;
}
