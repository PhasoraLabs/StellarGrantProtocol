// @ts-ignore
import { Parser } from 'json2csv';

export class ExportService {
  async exportCsv(grant: any, milestones: any[], contributions: any[]): Promise<string> {
    const data = milestones.map(m => ({
      GrantId: grant.id,
      GrantTitle: grant.title,
      Status: grant.status,
      Recipient: grant.recipient,
      TotalAmount: grant.totalAmount,
      MilestoneIdx: m.idx,
      MilestoneTitle: m.title,
      MilestoneDeadline: m.deadline,
    }));
    
    // if no milestones, at least output grant info
    if (data.length === 0) {
      data.push({
        GrantId: grant.id,
        GrantTitle: grant.title,
        Status: grant.status,
        Recipient: grant.recipient,
        TotalAmount: grant.totalAmount,
        MilestoneIdx: null as any,
        MilestoneTitle: null as any,
        MilestoneDeadline: null as any,
      });
    }

    const parser = new Parser();
    return parser.parse(data);
  }

  async exportPdf(grant: any, milestones: any[], contributions: any[]): Promise<Buffer> {
    const PdfPrinter = require('pdfmake');
    const fonts = {
      Helvetica: {
        normal: 'Helvetica',
        bold: 'Helvetica-Bold',
        italics: 'Helvetica-Oblique',
        bolditalics: 'Helvetica-BoldOblique'
      }
    };
    const printer = new PdfPrinter(fonts);

    const docDefinition = {
      defaultStyle: { font: 'Helvetica' },
      content: [
        { text: `Grant: ${grant.title}`, style: 'header' },
        { text: `Status: ${grant.status}` },
        { text: `Recipient: ${grant.recipient}` },
        { text: `Total Amount: ${grant.totalAmount}` },
        { text: ' ', margin: [0, 10, 0, 10] },
        { text: 'Milestones', style: 'subheader' },
        milestones.length > 0 ? {
          table: {
            headerRows: 1,
            widths: ['auto', '*', 'auto'],
            body: [
              ['Idx', 'Title', 'Deadline'],
              ...milestones.map(m => [m.idx?.toString() || '', m.title || '', m.deadline || ''])
            ]
          }
        } : { text: 'No milestones found.' }
      ],
      styles: {
        header: { fontSize: 18, bold: true, margin: [0, 0, 0, 10] },
        subheader: { fontSize: 14, bold: true, margin: [0, 10, 0, 5] }
      }
    };

    const pdfDoc = printer.createPdfKitDocument(docDefinition);
    return new Promise((resolve, reject) => {
      const chunks: Buffer[] = [];
      pdfDoc.on('data', (chunk: Buffer) => chunks.push(chunk));
      pdfDoc.on('end', () => resolve(Buffer.concat(chunks)));
      pdfDoc.on('error', reject);
      pdfDoc.end();
    });
  }
}
export const exportService = new ExportService();
