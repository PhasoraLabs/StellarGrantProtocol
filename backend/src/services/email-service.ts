/**
 * Escape text for safe interpolation into HTML email bodies (#1237).
 * User-supplied grant/milestone titles are rendered as literal text.
 */
export function escapeHtml(value: unknown): string {
  const s = value == null ? "" : String(value);
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

import sgMail from '@sendgrid/mail';

sgMail.setApiKey(process.env.SENDGRID_API_KEY!);

export type EmailEventType =
  | "milestone_submitted"
  | "milestone_approved"
  | "milestone_rejected"
  | "grant_funded"
  | "grant_created"
  | "milestone_deadline_upcoming"
  | "milestone_deadline_overdue";

export type EmailType = EmailEventType;

export interface EmailPayload {
  to: string;
  event: EmailEventType;
  data: Record<string, any>;
}

interface EmailTemplate {
  subject: string;
  html: string;
}

export function getEmailTemplate(type: EmailType, data: Record<string, any>): EmailTemplate {
  const base = process.env.FRONTEND_URL || "http://localhost:3000";
  const grantTitle = escapeHtml(data.grantTitle);
  const milestoneTitle = escapeHtml(data.milestoneTitle);
  const milestoneIdx = escapeHtml(data.milestoneIdx);
  const grantId = escapeHtml(data.grantId);
  const amount = escapeHtml(data.amount);
  const deadline = escapeHtml(data.deadline);
  const daysRemaining = escapeHtml(data.daysRemaining);
  const milestoneLabel = milestoneTitle || `#${milestoneIdx}`;
  const grantLabel = grantTitle || `Grant #${grantId}`;

  switch (type) {
    case 'milestone_approved':
      return {
        subject: `Milestone Approved: ${escapeHtml(data.grantTitle)}`,
        html: `<p>Congratulations! Your milestone <b>${milestoneTitle}</b> for grant <b>${grantTitle}</b> has been approved.</p>`
      };
    case 'milestone_submitted':
      return {
        subject: `New Milestone Submission: ${escapeHtml(data.grantTitle)}`,
        html: `<p>A new milestone <b>${milestoneTitle}</b> has been submitted for grant <b>${grantTitle}</b>. Please review it.</p>`
      };
    case 'milestone_rejected':
      return {
        subject: `Milestone Rejected: ${escapeHtml(data.grantTitle)}`,
        html: `<p>Your milestone <b>${milestoneTitle}</b> for grant <b>${grantTitle}</b> has been rejected.</p>`
      };
    case 'grant_funded':
      return {
        subject: `Grant Funded: ${escapeHtml(data.grantTitle)}`,
        html: `<p>Your grant <b>${grantTitle}</b> has been funded with ${amount}.</p>`
      };
    case 'grant_created':
      return {
        subject: `Grant Created: ${escapeHtml(data.grantTitle)}`,
        html: `<p>A new grant <b>${grantTitle}</b> has been created.</p>`
      };
    case 'milestone_deadline_upcoming':
      return {
        subject: `[StellarGrant] Milestone due in ${escapeHtml(data.daysRemaining)} day${Number(data.daysRemaining) === 1 ? "" : "s"} — Grant #${grantId}`,
        html: `<p>Your milestone <strong>${milestoneLabel}</strong> for <strong>${grantLabel}</strong> is due in <strong>${daysRemaining} day${Number(data.daysRemaining) === 1 ? "" : "s"}</strong>.</p>
               <p>Deadline: <code>${deadline}</code></p>
               <p><a href="${base}/grants/${grantId}/milestones">Open milestone dashboard</a></p>`
      };
    case 'milestone_deadline_overdue':
      return {
        subject: `[StellarGrant] Milestone overdue — Grant #${grantId}`,
        html: `<p>Your milestone <strong>${milestoneLabel}</strong> for <strong>${grantLabel}</strong> is now <strong>overdue</strong>.</p>
               <p>Deadline: <code>${deadline}</code></p>
               <p><a href="${base}/grants/${grantId}/milestones">Review milestone</a></p>`
      };
    default:
      throw new Error(`Unknown email type: ${type}`);
  }
}

interface SendEmailParams {
  to: string;
  subject: string;
  html: string;
}

export async function sendEmail({ to, subject, html }: SendEmailParams) {
  const msg = {
    to,
    from: process.env.SENDGRID_FROM_EMAIL!,
    subject,
    html,
  };
  await sgMail.send(msg);
}

export class EmailService {
  async send(payload: EmailPayload): Promise<void> {
    const template = getEmailTemplate(payload.event, payload.data);
    await sendEmail({
      to: payload.to,
      subject: template.subject,
      html: template.html,
    });
  }
}

export const emailService = new EmailService();
