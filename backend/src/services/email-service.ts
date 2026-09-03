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
  switch (type) {
    case 'milestone_approved':
      return {
        subject: `Milestone Approved: ${data.grantTitle}`,
        html: `<p>Congratulations! Your milestone <b>${data.milestoneTitle}</b> for grant <b>${data.grantTitle}</b> has been approved.</p>`
      };
    case 'milestone_submitted':
      return {
        subject: `New Milestone Submission: ${data.grantTitle}`,
        html: `<p>A new milestone <b>${data.milestoneTitle}</b> has been submitted for grant <b>${data.grantTitle}</b>. Please review it.</p>`
      };
    case 'milestone_rejected':
      return {
        subject: `Milestone Rejected: ${data.grantTitle}`,
        html: `<p>Your milestone <b>${data.milestoneTitle}</b> for grant <b>${data.grantTitle}</b> has been rejected.</p>`
      };
    case 'grant_funded':
      return {
        subject: `Grant Funded: ${data.grantTitle}`,
        html: `<p>Your grant <b>${data.grantTitle}</b> has been funded with ${data.amount}.</p>`
      };
    case 'grant_created':
      return {
        subject: `Grant Created: ${data.grantTitle}`,
        html: `<p>A new grant <b>${data.grantTitle}</b> has been created.</p>`
      };
    case 'milestone_deadline_upcoming':
      return {
        subject: `[StellarGrant] Milestone due in ${data.daysRemaining} day${Number(data.daysRemaining) === 1 ? "" : "s"} — Grant #${data.grantId}`,
        html: `<p>Your milestone <strong>${data.milestoneTitle ?? `#${data.milestoneIdx}`}</strong> for <strong>${data.grantTitle ?? `Grant #${data.grantId}`}</strong> is due in <strong>${data.daysRemaining} day${Number(data.daysRemaining) === 1 ? "" : "s"}</strong>.</p>
               <p>Deadline: <code>${data.deadline}</code></p>
               <p><a href="${base}/grants/${data.grantId}/milestones">Open milestone dashboard</a></p>`
      };
    case 'milestone_deadline_overdue':
      return {
        subject: `[StellarGrant] Milestone overdue — Grant #${data.grantId}`,
        html: `<p>Your milestone <strong>${data.milestoneTitle ?? `#${data.milestoneIdx}`}</strong> for <strong>${data.grantTitle ?? `Grant #${data.grantId}`}</strong> is now <strong>overdue</strong>.</p>
               <p>Deadline: <code>${data.deadline}</code></p>
               <p><a href="${base}/grants/${data.grantId}/milestones">Review milestone</a></p>`
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
