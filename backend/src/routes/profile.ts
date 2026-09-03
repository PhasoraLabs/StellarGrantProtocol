import { Router } from "express";
import { Repository } from "typeorm";
import { User } from "../entities/User";

export const buildProfileRouter = (userRepo: Repository<User>) => {
  const router = Router();

  router.get("/:address", async (req, res) => {
    const { address } = req.params;
    if (!address) {
      return res.status(400).json({ error: "Address is required" });
    }

    const user = await userRepo.findOneBy({ stellarAddress: address });
    
    // Default base profile shape
    const profile = {
      stellarAddress: address,
      verified: {
        github: false,
        githubUsername: null as string | null,
        twitter: false,
        twitterUsername: null as string | null,
      }
    };

    if (user) {
      if (user.githubId) {
        profile.verified.github = true;
        profile.verified.githubUsername = user.githubUsername ?? null;
      }
      if (user.twitterId) {
        profile.verified.twitter = true;
        profile.verified.twitterUsername = user.twitterUsername ?? null;
      }
    }

    res.json(profile);
  });

  return router;
};
