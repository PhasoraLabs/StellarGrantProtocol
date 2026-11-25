import express from "express";
import request from "supertest";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { Repository } from "typeorm";
import { User } from "../../src/entities/User";
import { buildUserRouter } from "../../src/routes/users";
import { DEFAULT_LIMIT, MAX_LIMIT } from "../../src/utils/pagination";

describe("routes/users pagination", () => {
  const sampleUsers: User[] = Array.from({ length: 5 }, (_, index) => ({
    id: index + 1,
    stellarAddress: `GUSER${String(index + 1).padStart(51, "0")}`,
    email: `user${index + 1}@example.com`,
    notifyMilestoneApproved: false,
    notifyMilestoneSubmitted: false,
  }));

  let findAndCountMock: ReturnType<typeof vi.fn>;
  let app: express.Express;

  beforeEach(() => {
    findAndCountMock = vi.fn(async (options?: { skip?: number; take?: number }) => {
      const skip = options?.skip ?? 0;
      const take = options?.take ?? DEFAULT_LIMIT;
      return [sampleUsers.slice(skip, skip + take), sampleUsers.length];
    });

    const userRepo = {
      findAndCount: findAndCountMock,
    } as unknown as Repository<User>;

    app = express();
    app.use(express.json());
    app.use("/users", buildUserRouter(userRepo));
  });

  it("returns paginated users with default limit and offset", async () => {
    const response = await request(app).get("/users");

    expect(response.status).toBe(200);
    expect(findAndCountMock).toHaveBeenCalledWith({
      order: { id: "ASC" },
      skip: 0,
      take: DEFAULT_LIMIT,
    });
    expect(response.body.data).toHaveLength(5);
    expect(response.body.meta).toEqual({
      total: 5,
      page: 1,
      limit: DEFAULT_LIMIT,
      totalPages: 1,
      hasNextPage: false,
      hasPrevPage: false,
      offset: 0,
    });
  });

  it("applies validated limit and offset query parameters", async () => {
    const response = await request(app).get("/users?limit=2&offset=2");

    expect(response.status).toBe(200);
    expect(findAndCountMock).toHaveBeenCalledWith({
      order: { id: "ASC" },
      skip: 2,
      take: 2,
    });
    expect(response.body.data.map((u: User) => u.id)).toEqual([3, 4]);
    expect(response.body.meta).toEqual({
      total: 5,
      page: 2,
      limit: 2,
      totalPages: 3,
      hasNextPage: true,
      hasPrevPage: true,
      offset: 2,
    });
  });

  it("supports page parameter when offset is omitted", async () => {
    const response = await request(app).get("/users?page=2&limit=2");

    expect(response.status).toBe(200);
    expect(findAndCountMock).toHaveBeenCalledWith({
      order: { id: "ASC" },
      skip: 2,
      take: 2,
    });
    expect(response.body.data.map((u: User) => u.id)).toEqual([3, 4]);
    expect(response.body.meta).toEqual({
      total: 5,
      page: 2,
      limit: 2,
      totalPages: 3,
      hasNextPage: true,
      hasPrevPage: true,
      offset: 2,
    });
  });

  it("returns empty data array when offset exceeds total records", async () => {
    const response = await request(app).get("/users?limit=2&offset=10");

    expect(response.status).toBe(200);
    expect(response.body.data).toEqual([]);
    expect(response.body.meta).toMatchObject({
      total: 5,
      limit: 2,
      offset: 10,
      totalPages: 3,
      hasNextPage: false,
    });
  });

  it("rejects invalid limit values with 400", async () => {
    const zeroLimit = await request(app).get("/users?limit=0");
    const overMaxLimit = await request(app).get(`/users?limit=${MAX_LIMIT + 1}`);
    const nonNumericLimit = await request(app).get("/users?limit=abc");

    expect(zeroLimit.status).toBe(400);
    expect(zeroLimit.body.error).toBe("Validation failed");
    expect(overMaxLimit.status).toBe(400);
    expect(nonNumericLimit.status).toBe(400);
    expect(findAndCountMock).not.toHaveBeenCalled();
  });

  it("rejects negative or non-integer offset values with 400", async () => {
    const negativeOffset = await request(app).get("/users?offset=-1");
    const floatOffset = await request(app).get("/users?offset=1.5");
    const nonNumericOffset = await request(app).get("/users?offset=xyz");

    expect(negativeOffset.status).toBe(400);
    expect(negativeOffset.body.error).toBe("Validation failed");
    expect(floatOffset.status).toBe(400);
    expect(nonNumericOffset.status).toBe(400);
    expect(findAndCountMock).not.toHaveBeenCalled();
  });

  it("rejects combining offset and page parameters with 400", async () => {
    const response = await request(app).get("/users?offset=0&page=1");

    expect(response.status).toBe(400);
    expect(response.body.error).toBe("Validation failed");
    expect(findAndCountMock).not.toHaveBeenCalled();
  });
});
