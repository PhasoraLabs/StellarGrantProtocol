import { Router } from "express";
import passport from "passport";
import { Repository } from "typeorm";
import { User } from "../entities/User";
import crypto from "crypto";

export const buildAuthRouter = (userRepo: Repository<User>) => {
  const router = Router();

  router.get("/github", (req, res, next) => {
    const stellarAddress = req.query.address as string;
    if (!stellarAddress) {
      return res.status(400).json({ error: "stellar address is required in query" });
    }
    
    // Generate nonce for PKCE/CSRF protection
    const state = crypto.randomBytes(16).toString("hex") + ":" + stellarAddress;
    (req.session as any).oauthState = state;

    passport.authenticate("github", { scope: ["user:email"], state })(req, res, next);
  });

  router.get(
    "/github/callback",
    passport.authenticate("github", { failureRedirect: "/login/error" }),
    async (req, res) => {
      const state = req.query.state as string;
      const sessionState = (req.session as any).oauthState;

      if (!state || state !== sessionState) {
        return res.status(400).json({ error: "Invalid state parameter. Possible CSRF." });
      }

      const stellarAddress = state.split(":")[1];
      const githubProfile = req.user as any;

      if (!stellarAddress || !githubProfile) {
        return res.status(400).json({ error: "Missing address or profile" });
      }

      let user = await userRepo.findOneBy({ stellarAddress });
      if (!user) {
        user = new User();
        user.stellarAddress = stellarAddress;
      }
      user.githubId = githubProfile.id;
      user.githubUsername = githubProfile.username;
      await userRepo.save(user);

      // Redirect back to frontend
      res.redirect(process.env.FRONTEND_URL ? `${process.env.FRONTEND_URL}/profile` : "/profile");
    }
  );

  router.get("/twitter", (req, res, next) => {
    const stellarAddress = req.query.address as string;
    if (!stellarAddress) {
      return res.status(400).json({ error: "stellar address is required in query" });
    }
    
    const state = crypto.randomBytes(16).toString("hex") + ":" + stellarAddress;
    (req.session as any).oauthState = state;

    // Twitter requires PKCE via express-session automatically if configured
    passport.authenticate("twitter", { state })(req, res, next);
  });

  router.get(
    "/twitter/callback",
    passport.authenticate("twitter", { failureRedirect: "/login/error" }),
    async (req, res) => {
      const state = req.query.state as string;
      const sessionState = (req.session as any).oauthState;

      if (!state || state !== sessionState) {
        return res.status(400).json({ error: "Invalid state parameter. Possible CSRF." });
      }

      const stellarAddress = state.split(":")[1];
      const twitterProfile = req.user as any;

      if (!stellarAddress || !twitterProfile) {
        return res.status(400).json({ error: "Missing address or profile" });
      }

      let user = await userRepo.findOneBy({ stellarAddress });
      if (!user) {
        user = new User();
        user.stellarAddress = stellarAddress;
      }
      user.twitterId = twitterProfile.id;
      user.twitterUsername = twitterProfile.username;
      await userRepo.save(user);

      res.redirect(process.env.FRONTEND_URL ? `${process.env.FRONTEND_URL}/profile` : "/profile");
    }
  );

  return router;
};
