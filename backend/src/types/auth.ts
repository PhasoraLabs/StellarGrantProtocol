import { Request } from "express";

export interface AuthUser {
  stellarAddress: string;
  // Add other user fields as needed
}

export interface AuthenticatedRequest extends Request {
  user?: AuthUser;
}
