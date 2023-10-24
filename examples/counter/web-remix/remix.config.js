/** @type {import('@remix-run/dev').AppConfig} */
module.exports = {
  ignoredRouteFiles: ["**/.*"],
  // appDirectory: "app",
  // assetsBuildDirectory: "public/build",
  // serverBuildPath: "build/index.js",
  // publicPath: "/build/",

  // make sure the server bundles our shared library
  serverDependenciesToBundle: [/^shared.*/],

  serverModuleFormat: "cjs",
};
