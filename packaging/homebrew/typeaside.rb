cask "typeaside" do
  version "0.2.1"
  sha256 :no_check

  url "https://typeaside.app/releases/v#{version}/TypeAside.dmg"
  name "TypeAside"
  desc "Native voice dictation"
  homepage "https://typeaside.app"

  livecheck do
    url "https://typeaside.app/releases/latest.json"
    strategy :json do |json|
      json["version"]
    end
  end

  depends_on macos: ">= :ventura"

  app "TypeAside.app"

  zap trash: [
    "~/Library/Application Support/typeaside",
    "~/Library/Caches/app.typeaside.dictation",
    "~/Library/Caches/typeaside",
    "~/Library/LaunchAgents/app.typeaside.plist",
    "~/Library/Preferences/app.typeaside.dictation.plist",
    "~/Library/Saved Application State/app.typeaside.dictation.savedState",
  ]
end
