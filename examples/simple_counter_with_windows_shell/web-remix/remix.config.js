/** @type {import('@remix-run/dev').AppConfig} */
module.exports = {
  ignoredRouteFiles: ["**/.*"],

  // make sure the server bundles our shared library
  serverDependenciesToBundle: [/^shared.*/],

  serverModuleFormat: "cjs",
};
