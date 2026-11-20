import { createRequire } from "node:module";
import path from "node:path";
import { describe, expect, it } from "vitest";

interface OpenApiOperation {
  summary?: string;
  responses?: Record<string, unknown>;
}

interface OpenApiDocument {
  paths: Record<
    string,
    Partial<Record<"get" | "post", OpenApiOperation>>
  >;
}

type SwaggerJsdocOptions = {
  definition: {
    openapi: string;
    info: { title: string; version: string };
  };
  apis: string[];
};

type SwaggerJsdoc = (options: SwaggerJsdocOptions) => OpenApiDocument;

const requireFromBackend = createRequire(
  path.resolve(process.cwd(), "package.json"),
);
const swaggerJsdoc = requireFromBackend("swagger-jsdoc") as SwaggerJsdoc;

describe("dispute OpenAPI documentation", () => {
  it("includes every dispute endpoint with a summary and responses", () => {
    const document = swaggerJsdoc({
      definition: {
        openapi: "3.0.0",
        info: { title: "StellarGrant API", version: "1.0.0" },
      },
      apis: [path.resolve(process.cwd(), "src/routes/disputes.ts")],
    });

    const operations = [
      ["/disputes", "get"],
      ["/disputes/{id}", "get"],
      ["/disputes/{id}/argument", "post"],
      ["/disputes/{id}/resolve", "post"],
      ["/grants/{grantId}/milestones/{idx}/dispute", "get"],
      ["/grants/{grantId}/milestones/{idx}/dispute", "post"],
    ] as const;

    for (const [endpoint, method] of operations) {
      const operation = document.paths[endpoint]?.[method];
      expect(operation?.summary).toBeTruthy();
      expect(Object.keys(operation?.responses ?? {})).not.toHaveLength(0);
    }
  });
});
