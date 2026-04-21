import "./globals.css";
import type { Metadata } from "next";
import { DM_Sans } from "next/font/google";

const dmSans = DM_Sans({ subsets: ["latin"] });

export const metadata: Metadata = {
  title: "Crux Weather",
  description: "Rust Core, TypeScript Shell (Next.js)",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" data-theme="light">
      <body className={dmSans.className}>{children}</body>
    </html>
  );
}
