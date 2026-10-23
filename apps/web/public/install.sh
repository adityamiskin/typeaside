#!/bin/sh
set -eu

main() {
    if [ "$(uname -s)" != Linux ] || [ "$(uname -m)" != x86_64 ]; then
        echo 'This installer supports Linux x86_64. See https://typeaside.app/download for other platforms.' >&2
        exit 1
    fi

    for command in curl mktemp; do
        command -v "$command" >/dev/null 2>&1 || {
            echo "Install $command and run this command again." >&2
            exit 1
        }
    done

    temporary=$(mktemp -d)
    staged_app=''
    trap 'rm -rf "$temporary"; if [ -n "$staged_app" ]; then rm -f "$staged_app"; fi' 0
    trap 'exit 1' HUP INT TERM
    public_key='RWSZYPHo0rUFPo3mvb5y2W+/Qkp/riKlC4w9GhJMcRXb4VB8dnDfnNyh'
    verifier=$(command -v minisign || true)
    if [ -z "$verifier" ]; then
        for command in sha256sum tar; do
            command -v "$command" >/dev/null 2>&1 || {
                echo "Install $command or minisign and run this command again." >&2
                exit 1
            }
        done
        echo 'Downloading signature verifier...'
        curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' --retry 3 \
            --output "$temporary/minisign.tar.gz" \
            https://github.com/jedisct1/minisign/releases/download/0.12/minisign-0.12-linux.tar.gz
        printf '%s  %s\n' 9a599b48ba6eb7b1e80f12f36b94ceca7c00b7a5173c95c3efc88d9822957e73 "$temporary/minisign.tar.gz" | sha256sum --check --status || {
            echo 'Signature verifier checksum mismatch. Installation stopped.' >&2
            exit 1
        }
        tar -xzf "$temporary/minisign.tar.gz" -C "$temporary" minisign-linux/x86_64/minisign
        verifier="$temporary/minisign-linux/x86_64/minisign"
    fi

    echo 'Downloading TypeAside...'
    for file in TypeAside.AppImage TypeAside.AppImage.minisig; do
        curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' --retry 3 \
            --output "$temporary/$file" "https://typeaside.app/releases/$file"
    done
    "$verifier" -V -P "$public_key" -m "$temporary/TypeAside.AppImage" -x "$temporary/TypeAside.AppImage.minisig"

    data_home=${XDG_DATA_HOME:-$HOME/.local/share}
    install_dir="$data_home/typeaside"
    bin_dir="$HOME/.local/bin"
    icon_dir="$data_home/icons/hicolor/scalable/apps"
    mkdir -p "$install_dir" "$bin_dir" "$icon_dir" "$data_home/applications"
    staged_app="$install_dir/.TypeAside.AppImage.$$"
    cp "$temporary/TypeAside.AppImage" "$staged_app"
    chmod 755 "$staged_app"
    mv -f "$staged_app" "$install_dir/TypeAside.AppImage"
    ln -sf "$install_dir/TypeAside.AppImage" "$bin_dir/typeaside"

    cat > "$icon_dir/typeaside.svg" <<'ICON'
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><rect width="256" height="256" rx="58" fill="#f9f9f8"/><g fill="#16171a"><path d="M86 122a56 56 0 0 1 112 0Z"/><path d="M58 134a56 56 0 0 0 112 0Z"/></g></svg>
ICON
    # Desktop entries decode escapes before parsing the quoted Exec argument.
    # shellcheck disable=SC2016
    desktop_exec=$(printf '%s' "$bin_dir/typeaside" | sed 's/\\/\\\\\\\\/g; s/"/\\\\"/g; s/\$/\\\\$/g; s/`/\\\\`/g; s/%/%%/g')
    cat > "$data_home/applications/typeaside.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=TypeAside
GenericName=Voice Dictation
Comment=Dictate anywhere and get clean, edited text
Keywords=dictation;voice;speech;transcribe;
Exec="$desktop_exec"
Icon=typeaside
Categories=Utility;
Terminal=false
DESKTOP
    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database "$data_home/applications" >/dev/null 2>&1 || true
    fi

    printf '\nInstalled TypeAside. Launch it from your applications menu.\n'
    case ":$PATH:" in
        *":$bin_dir:"*) echo 'Or run: typeaside' ;;
        *) printf 'Or run: "%s/typeaside"\nAdd %s to your PATH to use the typeaside command.\n' "$bin_dir" "$bin_dir" ;;
    esac
    echo 'Linux permissions: https://typeaside.app/docs/permissions#linux'
}

main "$@"
