/** @type {import('@bacons/apple-targets/app.plugin').ConfigFunction} */
module.exports = () => ({
  type: "keyboard",
  name: "TypeAsideKeyboard",
  displayName: "TypeAside",
  bundleIdentifier: ".keyboard",
  deploymentTarget: "17.0",
  entitlements: {
    "com.apple.security.application-groups": ["group.app.typeaside"],
  },
});
