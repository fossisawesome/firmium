#!/bin/bash
# Bump version across all project files and AUR packages

set -e

if [ -z "$1" ]; then
  echo "Usage: ./scripts/bump-version.sh <new-version>"
  echo "Example: ./scripts/bump-version.sh 1.5.0"
  exit 1
fi

NEW_VERSION="$1"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "🔄 Bumping version to $NEW_VERSION..."

# Update file based on type
update_file() {
  local file=$1
  local version=$2

  if [ ! -f "$file" ]; then
    echo "⚠️  File not found: $file"
    return
  fi

  if [[ $file == *.json ]]; then
    sed -i "s/\"version\": \"[^\"]*\"/\"version\": \"$version\"/" "$file"
  elif [[ $file == *.toml ]]; then
    sed -i "s/^version = \"[^\"]*\"/version = \"$version\"/" "$file"
  elif [[ $file == *PKGBUILD ]]; then
    sed -i "s/^pkgver=.*/pkgver=$version/" "$file"
    sed -i "s/^pkgrel=.*/pkgrel=1/" "$file"
  elif [[ $file == *.spec ]]; then
    sed -i "s/^Version:        .*/Version:        $version/" "$file"
    sed -i "s/^Release:        .*/Release:        1/" "$file"
    # Update %changelog: replace version in the most recent entry line
    CHANGELOG_DATE=$(date "+%a %b %d %Y")
    sed -i "s/^\* .* - .*$/\* $CHANGELOG_DATE fossisawesome <fossisawesome AT github DOT com> - $version-1/" "$file"
  elif [[ $file == *.md ]]; then
    sed -i "s/^\*\*Version\*\*: [^*]*/\*\*Version\*\*: $version/" "$file"
  elif [[ $file == *.kts ]]; then
    # Increment versionCode by 1 and set versionName to the new version
    CURRENT_CODE=$(grep -oP 'versionCode = \K[0-9]+' "$file")
    NEW_CODE=$((CURRENT_CODE + 1))
    sed -i "s/versionCode = [0-9]*/versionCode = $NEW_CODE/" "$file"
    sed -i "s/versionName = \"[^\"]*\"/versionName = \"$version\"/" "$file"
  fi

  echo "✓ Updated: $file"
}

# Update main repo files
update_file "$PROJECT_ROOT/CLAUDE.md" "$NEW_VERSION"
update_file "$PROJECT_ROOT/Cargo.toml" "$NEW_VERSION"
update_file "$PROJECT_ROOT/PKGBUILD" "$NEW_VERSION"
update_file "$PROJECT_ROOT/firmium.spec" "$NEW_VERSION"

# Update Android app
update_file "$PROJECT_ROOT/android/app/build.gradle.kts" "$NEW_VERSION"

# Update AUR folders
update_file "$HOME/firmium/aur-firmium-git/PKGBUILD" "$NEW_VERSION"
update_file "$HOME/firmium/aur-firmium-bin/PKGBUILD" "$NEW_VERSION"

# Update .SRCINFO in AUR folders if they exist
for aur_dir in "$HOME/firmium/aur-firmium-git" "$HOME/firmium/aur-firmium-bin"; do
  if [ -d "$aur_dir" ] && [ -f "$aur_dir/PKGBUILD" ]; then
    cd "$aur_dir"
    makepkg --printsrcinfo > .SRCINFO 2>/dev/null || echo "⚠️  Could not generate .SRCINFO for $aur_dir"
  fi
done

echo "✅ Version bumped to $NEW_VERSION"
echo ""
echo "Next steps:"
echo "  1. Commit changes: git add . && git commit -m 'chore: bump to v$NEW_VERSION' && git push"
echo "  2. Tag: git tag v$NEW_VERSION && git push origin tag v$NEW_VERSION"
