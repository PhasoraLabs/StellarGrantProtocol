import { MetadataValidationError, MetadataValidationIssue } from "./errors/MetadataValidationError";
import { IpfsMetadataSchemaName } from "./types";

type JsonSchema = {
  $id: IpfsMetadataSchemaName;
  type: "object";
  required: string[];
  properties: Record<string, unknown>;
  additionalProperties: boolean;
};

export const GRANT_METADATA_SCHEMA: JsonSchema = {
  $id: "grant",
  type: "object",
  required: ["title", "description"],
  properties: {
    title: { type: "string", minLength: 1 },
    description: { type: "string", minLength: 1 },
    summary: { type: "string" },
    category: { type: "string" },
    links: { type: "array", items: { type: "string" } },
    milestones: { type: "array" },
  },
  additionalProperties: true,
};

export const MILESTONE_METADATA_SCHEMA: JsonSchema = {
  $id: "milestone",
  type: "object",
  required: ["title", "description", "acceptanceCriteria"],
  properties: {
    title: { type: "string", minLength: 1 },
    description: { type: "string", minLength: 1 },
    acceptanceCriteria: { type: "string", minLength: 1 },
    proofCid: { type: "string" },
    links: { type: "array", items: { type: "string" } },
    milestoneIdx: { type: "number" },
  },
  additionalProperties: true,
};

export const IPFS_METADATA_SCHEMAS: Record<IpfsMetadataSchemaName, JsonSchema> = {
  grant: GRANT_METADATA_SCHEMA,
  milestone: MILESTONE_METADATA_SCHEMA,
};

export function inferMetadataSchemaName(metadata: Record<string, unknown>): IpfsMetadataSchemaName {
  if (
    "acceptanceCriteria" in metadata ||
    "milestoneIdx" in metadata ||
    "proofCid" in metadata
  ) {
    return "milestone";
  }
  return "grant";
}

export function validateMetadataAgainstSchema(
  schemaName: IpfsMetadataSchemaName,
  metadata: Record<string, unknown>,
): void {
  const schema = IPFS_METADATA_SCHEMAS[schemaName];
  const issues: MetadataValidationIssue[] = [];

  for (const field of schema.required) {
    const value = metadata[field];
    if (value === undefined || value === null || value === "") {
      issues.push({
        field,
        message: "is required",
      });
    }
  }

  for (const [field, value] of Object.entries(metadata)) {
    if (value === undefined || value === null) continue;
    if (typeof value === "string" && value.trim() === "") {
      issues.push({
        field,
        message: "must not be an empty string",
      });
    }
  }

  if (issues.length > 0) {
    throw new MetadataValidationError(schemaName, issues);
  }
}
