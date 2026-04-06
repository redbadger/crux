import "bulma/css/bulma.min.css";
import "./globals.css";
import type { Metadata } from "next";
import { DM_Sans } from "next/font/google";
import Script from "next/script";

const dmSans = DM_Sans({ subsets: ["latin"] });

export const metadata: Metadata = {
  title: "Crux Weather Example - NextJS",
  description: "Rust Core, TypeScript Shell (NextJS)",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" data-theme="light">
      <head>
        <Script
          src="https://unpkg.com/@phosphor-icons/web"
          strategy="beforeInteractive"
        />
      </head>
      <body className={dmSans.className}>{children}</body>
    </html>
  );
}
