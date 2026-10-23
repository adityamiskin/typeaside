import { useState } from "react";
import { Button } from "@typeaside/ui/components/button";

const command = "curl -fsSL https://typeaside.app/install.sh | sh";

export function InstallCommand() {
  const [copied, setCopied] = useState(false);
  const [failed, setFailed] = useState(false);

  async function copy() {
    try {
      await navigator.clipboard.writeText(command);
      setCopied(true);
      setFailed(false);
    } catch {
      setFailed(true);
    }
  }

  return (
    <div className="install-command">
      <div className="install-command-line">
        <code>{command}</code>
        <Button variant="ghost" size="sm" onClick={copy} aria-label="Copy Linux install command">
          {copied ? "Copied" : "Copy"}
        </Button>
      </div>
      <span className="sr-only" role="status">
        {copied ? "Linux install command copied" : ""}
      </span>
      {failed && <p role="alert">Couldn’t copy. Select and copy the command above.</p>}
    </div>
  );
}
