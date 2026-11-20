import { StellarGrantsError } from "./StellarGrantsError";

export type MetadataValidationIssue = {
  field: string;
  message: string;
};

export class MetadataValidationError extends StellarGrantsError {
  readonly issues: MetadataValidationIssue[];
  readonly schema: string;

  constructor(schema: string, issues: MetadataValidationIssue[]) {
    const details = issues
      .map((issue) => `${issue.field}: ${issue.message}`)
      .join(", ");
    super(
      `IPFS metadata validation failed for schema \"${schema}\": ${details}`,
      "METADATA_SCHEMA_VALIDATION_FAILED",
      { schema, issues },
    );
    this.name = "MetadataValidationError";
    this.schema = schema;
    this.issues = issues;
  }
}
