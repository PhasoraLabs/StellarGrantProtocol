import { Router } from "express";
import { Repository } from "typeorm";
import { Keypair, StrKey } from "@stellar/stellar-sdk";
import { Contributor } from "../entities/Contributor";
import { Grant } from "../entities/Grant";
import { validateBody, validateParams } from "../middlewares/validation-middleware";
import { profileUpdateSchema, addressParamSchema } from "../schemas";

const MAX_SKEW_MS = 5 * 60 * 1000;

function buildProfileIntentMessage(payload: {
  address: string;
  nonce: string;
  timestamp: number;
  patch: Record<string, unknown>;
}): string {
  // canonical payload for signing: stable field order via JSON.stringify on a reduced object
  const patchJson = JSON.stringify(payload.patch);
  return [
    "stellargrant:profile_update:v1",
    payload.address,
    payload.nonce,
    payload.timestamp,
    "PATCH:/profiles/me",
    patchJson,
  ].join("|");
}

function verifySignature(params: {
  address: string;
  signature: string;
  message: string;
}): boolean {
  if (!StrKey.isValidEd25519PublicKey(params.address)) return false;
  const keypair = Keypair.fromPublicKey(params.address);
  return keypair.verify(
    Buffer.from(params.message, "utf8"),
    Buffer.from(params.signature, "base64"),
  );
}

function toProfile(c: Contributor, grants?: Grant[]) {
  return {
    address: c.address,
    bio: c.bio ?? null,
    profilePictureUrl: c.profilePictureUrl ?? null,
    githubUrl: c.githubUrl ?? null,
    twitterUrl: c.twitterUrl ?? null,
    linkedinUrl: c.linkedinUrl ?? null,
    updatedAt: c.updatedAt,
    grants: grants?.map(g => ({
      id: g.id,
      title: g.title,
      status: g.status,
      totalAmount: g.totalAmount,
      tags: g.tags,
    })) ?? [],
  };
}

export const buildProfilesRouter = (contributorRepo: Repository<Contributor>, grantRepo: Repository<Grant>) => {
  const router = Router();

  router.get("/:address", validateParams(addressParamSchema), async (req, res, next) => {
    try {
      const { address } = (req as any).validatedParams;

      const contributor = await contributorRepo.findOne({ where: { address } });
      if (!contributor) {
        res.status(404).json({ error: "Profile not found" });
        return;
      }

      const grants = await grantRepo.find({ where: { recipient: address } });
      res.json({ data: toProfile(contributor, grants) });
    } catch (error) {
      next(error);
    }
  });

  router.patch("/me", validateBody(profileUpdateSchema), async (req, res, next) => {
    try {
      const { address, signature, nonce, timestamp, ...fields } = (req as any).validatedBody;
      if (!StrKey.isValidEd25519PublicKey(address)) {
        res.status(400).json({ error: "Invalid Stellar address" });
        return;
      }

      if (Math.abs(Date.now() - timestamp) > MAX_SKEW_MS) {
        res.status(400).json({ error: "Expired intent timestamp" });
        return;
      }

      const patch: Record<string, unknown> = {};
      for (const [k, v] of Object.entries(fields)) {
        if (v !== undefined) patch[k] = v;
      }

      const message = buildProfileIntentMessage({ address, nonce, timestamp, patch });
      const ok = verifySignature({ address, signature, message });
      if (!ok) {
        res.status(401).json({ error: "Invalid signature" });
        return;
      }

      let contributor = await contributorRepo.findOne({ where: { address } });
      if (!contributor) {
        contributor = contributorRepo.create({
          address,
          reputation: 0,
          totalGrantsCompleted: 0,
          isBlacklisted: false,
          email: null,
          emailNotifications: true,
          bio: null,
          profilePictureUrl: null,
          githubUrl: null,
          twitterUrl: null,
          linkedinUrl: null,
        });
      }

      if ("bio" in patch) contributor.bio = (patch.bio as string | null) ?? null;
      if ("profilePictureUrl" in patch) contributor.profilePictureUrl = (patch.profilePictureUrl as string | null) ?? null;
      if ("githubUrl" in patch) contributor.githubUrl = (patch.githubUrl as string | null) ?? null;
      if ("twitterUrl" in patch) contributor.twitterUrl = (patch.twitterUrl as string | null) ?? null;
      if ("linkedinUrl" in patch) contributor.linkedinUrl = (patch.linkedinUrl as string | null) ?? null;

      const saved = await contributorRepo.save(contributor);
      res.json({ data: toProfile(saved) });
    } catch (error) {
      next(error);
    }
  });

  return router;
};

