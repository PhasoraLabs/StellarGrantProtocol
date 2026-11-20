import type { RequestHandler } from "express";

export type Role = "admin" | "reviewer" | "applicant";

export function requireRole(...allowed: Role[]): RequestHandler {
  return (req, res, next) => {
    const role = req.header("x-role") as Role | undefined;
    if (!role) { res.status(401).json({ error: "Authentication required" }); return; }
    if (!allowed.includes(role)) { res.status(403).json({ error: "Insufficient role" }); return; }
    next();
  };
}
