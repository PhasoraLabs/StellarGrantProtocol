import request from "supertest";
import { beforeAll, afterAll, describe, expect, it } from "vitest";
import { DataSource } from "typeorm";
import { createApp } from "../../src/app";
import { buildDataSource } from "../../src/db/data-source";
import { MockSorobanContractClient } from "../../src/soroban/mock-client";

describe("Localization API e2e", () => {
  let dataSource: DataSource;
  const sorobanClient = new MockSorobanContractClient();

  beforeAll(async () => {
    dataSource = buildDataSource("sqljs://memory");
    await dataSource.initialize();
  });

  afterAll(async () => {
    if (dataSource?.isInitialized) {
      await dataSource.destroy();
    }
  });

  it("returns English content by default", async () => {
    const app = createApp(dataSource, sorobanClient);
    const response = await request(app).get("/grants/1");

    expect(response.status).toBe(200);
    expect(response.body.data.title).toBe("Open Source Grants Q2");
    expect(response.body.data.description).toBe("Supporting the best open-source projects.");
  });

  it("returns Spanish content when Accept-Language is set to es", async () => {
    const app = createApp(dataSource, sorobanClient);
    const response = await request(app)
      .get("/grants/1")
      .set("Accept-Language", "es");

    expect(response.status).toBe(200);
    expect(response.body.data.title).toBe("Subvenciones de Código Abierto Q2");
    expect(response.body.data.description).toBe("Apoyando los mejores proyectos de código abierto.");
  });

  it("falls back to English for missing translations (e.g., fr)", async () => {
    const app = createApp(dataSource, sorobanClient);
    const response = await request(app)
      .get("/grants/1")
      .set("Accept-Language", "fr");

    expect(response.status).toBe(200);
    expect(response.body.data.title).toBe("Open Source Grants Q2");
    expect(response.body.data.description).toBe("Supporting the best open-source projects.");
  });

  it("falls back to English when the requested language is partially missing (Grant 2)", async () => {
    // Grant 2 only has 'en' translation in mock
    const app = createApp(dataSource, sorobanClient);
    const response = await request(app)
      .get("/grants/2")
      .set("Accept-Language", "es");

    expect(response.status).toBe(200);
    expect(response.body.data.title).toBe("Climate Data Tools");
    expect(response.body.data.description).toBe("Tools for measuring climate impact.");
  });

  it("localizes list of grants", async () => {
    const app = createApp(dataSource, sorobanClient);
    const response = await request(app)
      .get("/grants")
      .set("Accept-Language", "es");

    expect(response.status).toBe(200);
    const grant1 = response.body.data.find((g: any) => g.id === 1);
    expect(grant1.title).toBe("Subvenciones de Código Abierto Q2");
  });
});
