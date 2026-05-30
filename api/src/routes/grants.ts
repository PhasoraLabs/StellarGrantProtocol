import { Router } from "express";
import { Repository } from "typeorm";
import { Grant } from "../entities/Grant";
import { GrantSyncService } from "../services/grant-sync-service";

const translations: Record<string, Record<number, { title: string; description: string }>> = {
  es: {
    1: {
      title: "Subvenciones de Código Abierto Q2",
      description: "Apoyando los mejores proyectos de código abierto.",
    },
  },
};

const defaultGrantsData: Record<number, { title: string; description: string }> = {
  1: {
    title: "Open Source Grants Q2",
    description: "Supporting the best open-source projects.",
  },
  2: {
    title: "Climate Data Tools",
    description: "Tools for measuring climate impact.",
  },
};

function localizeGrant(grant: Grant, lang?: string): any {
  const grantId = grant.id;
  const defaults = defaultGrantsData[grantId] || { title: grant.title, description: grant.description || "" };
  
  const localized = {
    ...grant,
    title: defaults.title,
    description: defaults.description || null,
  };

  if (lang && translations[lang] && translations[lang][grantId]) {
    const translation = translations[lang][grantId];
    if (translation.title) localized.title = translation.title;
    if (translation.description) localized.description = translation.description;
  }

  return localized;
}

export const buildGrantRouter = (grantRepo: Repository<Grant>, syncService: GrantSyncService) => {
  const router = Router();

  router.get("/", async (req, res, next) => {
    try {
      await syncService.syncAllGrants();
      const communityId = req.query.communityId !== undefined ? Number(req.query.communityId) : undefined;
      const grants = Number.isInteger(communityId)
        ? await grantRepo.find({ where: { communityId }, order: { id: "ASC" } })
        : await grantRepo.find({ order: { id: "ASC" } });
      const lang = req.header("Accept-Language");
      const localizedGrants = grants.map((g) => localizeGrant(g, lang));
      res.json({ data: localizedGrants });
    } catch (error) {
      next(error);
    }
  });

  router.get("/:id", async (req, res, next) => {
    try {
      const id = Number(req.params.id);
      if (Number.isNaN(id)) {
        res.status(400).json({ error: "Invalid grant id" });
        return;
      }

      await syncService.syncGrant(id);
      const grant = await grantRepo.findOne({ where: { id } });

      if (!grant) {
        res.status(404).json({ error: "Grant not found" });
        return;
      }

      const lang = req.header("Accept-Language");
      res.json({ data: localizeGrant(grant, lang) });
    } catch (error) {
      next(error);
    }
  });

  return router;
};
