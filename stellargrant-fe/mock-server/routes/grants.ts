import { Router } from "express";
import { grants } from "../fixtures/grants";

const router = Router();

// GET /grants - Paginated and filtered list
router.get("/", (req, res) => {
  const page = parseInt(req.query.page as string) || 1;
  const limit = parseInt(req.query.limit as string) || 12;
  const status = req.query.status as string;
  const token = req.query.token as string;
  const q = (req.query.q as string)?.toLowerCase();

  let filteredGrants = [...grants];

  if (status) {
    const statuses = status.split(",").map(s => s.trim().toLowerCase());
    filteredGrants = filteredGrants.filter(g => statuses.includes(g.status.toLowerCase()));
  }

  if (token) {
    filteredGrants = filteredGrants.filter(g => g.token.toLowerCase() === token.toLowerCase());
  }

  if (q) {
    filteredGrants = filteredGrants.filter(g => 
      g.title.toLowerCase().includes(q) || 
      g.description.toLowerCase().includes(q)
    );
  }

  const startIndex = (page - 1) * limit;
  const endIndex = page * limit;
  const paginatedGrants = filteredGrants.slice(startIndex, endIndex);

  res.json({
    data: paginatedGrants,
    total: filteredGrants.length,
    page,
    limit,
    nextPage: endIndex < filteredGrants.length ? page + 1 : null,
  });
});

// GET /grants/:id - Single grant
router.get("/:id", (req, res) => {
  const grant = grants.find(g => g.id === req.params.id);
  if (!grant) {
    return res.status(404).json({ error: "Grant not found" });
  }
  res.json({ data: grant });
});

// GET /grants/:id/funders - Mock funders
router.get("/:id/funders", (req, res) => {
  const funders = [
    { address: "GD...FUND1", amount: 10000, token: "XLM" },
    { address: "GD...FUND2", amount: 5000, token: "XLM" },
    { address: "GD...FUND3", amount: 2500, token: "XLM" }
  ];
  res.json({ data: funders });
});

export default router;
