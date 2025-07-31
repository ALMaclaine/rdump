#!/bin/bash
set -eo pipefail

# --- Configuration ---
CARGO_TOML_PATH="rdump/Cargo.toml"
CARGO_LOCK_PATH="rdump/Cargo.lock"

# --- Helper Functions ---
function check_command() {
  if ! command -v "$1" &> /dev/null; then
    echo "Error: Required command '$1' is not installed. Please install it and try again."
    exit 1
  fi
}

function check_git_clean() {
  if ! git diff-index --quiet HEAD --; then
    echo "Error: Git working directory is not clean. Please commit or stash your changes."
    exit 1
  fi
}

# --- Main Script ---
check_command "git"
check_command "cargo"
check_command "gh"
check_git_clean

# 1. Get release type from argument
BUMP_TYPE="$1"
if [[ "$BUMP_TYPE" != "patch" && "$BUMP_TYPE" != "minor" && "$BUMP_TYPE" != "major" ]]; then
  echo "Usage: ./scripts/release.sh [patch|minor|major]"
  exit 1
fi

# 2. Read current version from Cargo.toml
CURRENT_VERSION=$(grep "^version" "$CARGO_TOML_PATH" | sed -E 's/version = "([^"]+)"/\1/')
echo "Current version: $CURRENT_VERSION"

# 3. Calculate the new version
IFS='.' read -r -a V <<< "$CURRENT_VERSION"
case "$BUMP_TYPE" in
  "patch") V[2]=$((V[2] + 1));;
  "minor") V[1]=$((V[1] + 1)); V[2]=0;;
  "major") V[0]=$((V[0] + 1)); V[1]=0; V[2]=0;;
esac
NEW_VERSION="${V[0]}.${V[1]}.${V[2]}"
TAG="v$NEW_VERSION"

# 4. Confirm with the user
echo "This script will perform the following actions:"
echo "  - Bump version from $CURRENT_VERSION to $NEW_VERSION"
echo "  - Publish version $NEW_VERSION to crates.io"
echo "  - Create and push git tag '$TAG'"
echo "  - Create a GitHub release for '$TAG'"
read -p "Are you sure you want to continue? (y/n) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
  echo "Release cancelled."
  exit 1
fi

# 5. Update Cargo.toml and Cargo.lock
echo "Updating $CARGO_TOML_PATH to version $NEW_VERSION..."
sed -i.bak "s/^version = \".*\"/version = \"$NEW_VERSION\"/" "$CARGO_TOML_PATH"
rm "${CARGO_TOML_PATH}.bak"

# This command will update Cargo.lock based on the new version in Cargo.toml
cargo check --manifest-path "$CARGO_TOML_PATH"

# 6. Commit the version bump
echo "Committing version bump..."
git add "$CARGO_TOML_PATH" "$CARGO_LOCK_PATH"
git commit -m "chore(release): $TAG"

# 7. Publish to crates.io
echo "Publishing to crates.io..."
cargo publish --manifest-path "$CARGO_TOML_PATH"

# 8. Push commit and create tag/release on GitHub
echo "Pushing commit to GitHub..."
git push

echo "Creating GitHub release..."
gh release create "$TAG" --generate-notes --title "$TAG"

echo "✅ Release $TAG successfully published!"
echo "The GitHub Action workflow will now build binaries and attach them to the release."

