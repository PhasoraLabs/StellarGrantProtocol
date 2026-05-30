import { Router, Response } from "express";
import { FeeCollection } from "../entities/FeeCollection";
import { Grant } from "../entities/Grant";
import { DataSource, In } from "typeorm";
import { authMiddleware } from "../middlewares/auth-middleware";
import { AuthenticatedRequest } from "../types/auth";

export function buildMyDonationsRouter(dataSource: DataSource) {
  const router = Router();

  // GET /?token=TOKEN_SYMBOL
  router.get("/", authMiddleware, async (req: AuthenticatedRequest, res: Response) => {
    const funderAddress = req.user?.stellarAddress;

    const token = req.query.token as string | undefined;
    const feeRepo = dataSource.getRepository(FeeCollection);

    // Build query
    const where: any = { funderAddress };
    if (token) where.token = token;

    const donations = await feeRepo.find({ where });

    // Aggregate totals per token
    const totals: Record<string, string> = {};
    donations.forEach((d: any) => {
      if (!totals[d.token]) totals[d.token] = "0";
      totals[d.token] = (BigInt(totals[d.token]) + BigInt(d.totalContribution)).toString();
    });

    // Optionally, join grant info
    const grantIds = [...new Set(donations.map((d: any) => d.grantId))];
    const grantRepo = dataSource.getRepository(Grant);
    const grants = await grantRepo.find({ where: { id: grantIds.length > 0 ? In(grantIds) : undefined } });
    const grantMap = Object.fromEntries(grants.map((g: any) => [g.id, g]));

    res.json({
      donations: donations.map((d: any) => ({
        grant: grantMap[d.grantId] || null,
        token: d.token,
        amount: d.totalContribution,
        createdAt: d.createdAt,
      })),
      totals,
    });
  });

  return router;
}
