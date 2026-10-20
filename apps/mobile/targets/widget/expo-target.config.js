/** @type {import('@bacons/apple-targets/app.plugin').ConfigFunction} */
module.exports = () => ({
  type: "widget",
  name: "TypeAsideWidgets",
  displayName: "TypeAside",
  bundleIdentifier: ".widgets",
  deploymentTarget: "17.0",
  entitlements: {
    "com.apple.security.application-groups": ["group.app.typeaside"],
  },
});
