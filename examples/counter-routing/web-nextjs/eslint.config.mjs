import nextConfig from "eslint-config-next/core-web-vitals";

const config = [{ ignores: ["generated/"] }, ...nextConfig];
export default config;
