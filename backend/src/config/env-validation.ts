/**
 * Environment Variable Validation
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
    name: "PORT",
    required: false,
    defaultValue: "4000",
    description: "API server port",
  },
  {
    name: "DATABASE_URL",
    required: false,
    defaultValue: "postgres://postgres:postgres@localhost:5432/stellargrant",
    description: "PostgreSQL database connection string",
  },
  {
    name: "NODE_ENV",
    required: false,
    defaultValue: "development",
    description: "Application environment (development, production, test)",
  },
];

interface ValidationResult {
  valid: boolean;
  errors: string[];
  warnings: string[];
}

export function validateEnv(): ValidationResult {
  const errors: string[] = [];
  const warnings: string[] = [];
  const isDevelopment = process.env.NODE_ENV !== "production";

  for (const config of ENV_VARS) {
    const value = process.env[config.name];

    if (!value) {
      if (config.required) {
        errors.push(
          `Missing required environment variable: ${config.name}\n` +
          `  Description: ${config.description}\n` +
          `  Please set ${config.name} in your .env file or environment.`
        );
      } else if (config.defaultValue) {
        if (isDevelopment) {
          warnings.push(
            `Using default value for ${config.name}: ${config.defaultValue}\n` +
            `  Description: ${config.description}\n` +
            `  Consider setting ${config.name} explicitly in your .env file.`
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
  };
}

export function logValidationResult(result: ValidationResult): void {
  const isDevelopment = process.env.NODE_ENV !== "production";

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

export function validateEnvOnStartup(): void {
  const result = validateEnv();
  logValidationResult(result);

  if (!result.valid) {
    const isDevelopment = process.env.NODE_ENV !== "production";
    
    if (isDevelopment) {
      // In development, fail fast with clear error messages
      console.error("Startup aborted due to missing required environment variables.");
      process.exit(1);
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
}
