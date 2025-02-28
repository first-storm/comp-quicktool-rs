#!/bin/bash

echo "Starting installation of QuickTool (will replace 1091)..."

# Check and create ~/bin directory
if [ ! -d "$HOME/bin" ]; then
    echo "Creating $HOME/bin directory..."
    mkdir -p "$HOME/bin"
fi

# Remove existing 1091 if present
if [ -f "$HOME/bin/1091" ]; then
    echo "Removing existing 1091..."
    rm "$HOME/bin/1091"
fi

# Download the tool and rename it
echo "Downloading the tool..."
curl -L https://github.com/first-storm/comp-quicktool-rs/releases/download/0.0.1/quicktool -o "$HOME/bin/1091"

# Give execution permission
echo "Setting execution permission..."
chmod +x "$HOME/bin/1091"

# Add to PATH (if not already added)
if ! grep -q 'export PATH="$HOME/bin:$PATH"' "$HOME/.bashrc"; then
    echo "Adding to PATH environment variable..."
    echo 'export PATH="$HOME/bin:$PATH"' >> "$HOME/.bashrc"
fi

# Make PATH setting effective immediately
echo "Making environment variable effective immediately..."
export PATH="$HOME/bin:$PATH"

echo "Installation complete! Running help command..."
1091 help