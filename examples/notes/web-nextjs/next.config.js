const WindiCSSWebpackPlugin = require("windicss-webpack-plugin");

/** @type {import('next').NextConfig} */
const nextConfig = {
  webpack(nextConfig) {
    nextConfig.plugins.push(new WindiCSSWebpackPlugin());

    return nextConfig;
  },
};

module.exports = nextConfig;
