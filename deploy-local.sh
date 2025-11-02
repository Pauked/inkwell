#!/bin/bash
#
# Deploy inkwell locally to Dropbox Utils folder
# Usage: ./deploy-local.sh [target_path]
# Default: $DROPBOX_PATH/Utils/inkwell or ~/Desktop/inkwell
# Make executable: chmod +x deploy-local.sh

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🔨 Building inkwell in release mode...${NC}"
if cargo build --release; then
    echo -e "${GREEN}✅ Build successful${NC}"
else
    echo -e "${RED}❌ Build failed${NC}"
    exit 1
fi

# Source paths
source_app="./target/release/inkwell"

# Determine target path (allow override via command line)
if [ -n "$1" ]; then
    target_path="$1"
elif [ -n "$DROPBOX_PATH" ]; then
    target_path="$DROPBOX_PATH/Utils/inkwell"
else
    target_path="$HOME/Desktop/inkwell"
fi

echo -e "${BLUE}📁 Target deployment path: $target_path${NC}"

# Create target directory if it doesn't exist
if [ ! -d "$target_path" ]; then
    echo -e "${YELLOW}📂 Creating deployment directory...${NC}"
    mkdir -p "$target_path"
fi

# Check if source files exist
if [ ! -f "$source_app" ]; then
    echo -e "${RED}❌ Binary not found: $source_app${NC}"
    exit 1
fi

# Copy to deploy folder
echo -e "${BLUE}📦 Deploying files...${NC}"
cp "$source_app" "$target_path/inkwell" || exit 1

# Create sample config.toml if it doesn't exist
if [ ! -f "$target_path/config.toml" ]; then
    echo -e "${YELLOW}📝 Creating sample config.toml configuration...${NC}"
    cat > "$target_path/config.toml" << 'EOF'
# Inkwell Configuration
# Configure your default export folder for converted Kindle notebooks

# Default folder where markdown files will be saved
# Update this path to match your Obsidian vault or preferred location
default_export_folder = "$HOME/Documents/Obsidian/Book Highlights"
EOF
else
    echo -e "${GREEN}✓ Existing config.toml found, skipping...${NC}"
fi

# Create a README if it doesn't exist
if [ ! -f "$target_path/README.txt" ]; then
    echo -e "${YELLOW}📝 Creating README.txt...${NC}"
    cat > "$target_path/README.txt" << 'EOF'
Inkwell - Kindle to Obsidian Converter
======================================

Usage:
------
1. Edit config.toml and set your default_export_folder path
2. Export your Kindle notebook as HTML from the Kindle macOS app
3. Run: ./inkwell "/path/to/Book - Notebook.html"

The converted markdown file will be saved to your configured export folder.

Options:
--------
# Convert to default folder (from config.toml)
./inkwell "/path/to/Book - Notebook.html"

# Specify custom output file
./inkwell "/path/to/Book - Notebook.html" -o "/custom/path/output.md"

# Override export directory temporarily
./inkwell "/path/to/Book - Notebook.html" -d "/different/folder"

Configuration:
--------------
The config.toml file should be in the same directory as the inkwell binary.
If not found, inkwell will look for ~/.config/inkwell/config.toml

For more information, visit: https://github.com/yourusername/inkwell
EOF
fi

echo -e "${GREEN}✅ Deployment successful!${NC}"
echo ""
echo -e "${BLUE}📋 Deployed to: $target_path${NC}"
echo -e "${BLUE}📋 Binary: inkwell${NC}"
echo ""
echo -e "${YELLOW}Next steps:${NC}"
echo -e "  1. Edit $target_path/config.toml with your export folder path"
echo -e "  2. Run: cd \"$target_path\" && ./inkwell \"/path/to/Notebook.html\""
echo ""
