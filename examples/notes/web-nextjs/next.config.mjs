// @ts-check

/**
 * @type {import('next').NextConfig}
 */
const nextConfig = {
  /* config options here */
  turbopack: {
    // Empty config to acknowledge Turbopack usage and silence webpack config warning
  },
  webpack: (config, { isServer }) => {
    // Fallback webpack configuration for when webpack mode is used
    config.resolve.fallback = {
      ...config.resolve.fallback,
      fs: false,
      path: false,
    };

    return config;
  },
  // Transpile the shared_types and shared packages to ensure compatibility
  transpilePackages: ["shared_types", "shared"],
};

export default nextConfig;
