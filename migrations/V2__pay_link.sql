-- Add optional pay_link URL to invoices. When set, the renderer encodes it
-- as a QR code on the generated PDF (via the qrcode crate + qr-render Typst
-- component). Typical use: paste a Stripe Payment Link URL here.

ALTER TABLE invoices ADD COLUMN pay_link TEXT;
