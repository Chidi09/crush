# Installation

Install Crush across Windows, macOS, Linux, and cloud servers.

## Quick Install (Linux & macOS)

To install Crush on Linux or macOS, run the following command in your terminal:

```bash
curl -fsSL https://crushrun.dev/install | sh
```

---

## Linux Package Managers

### Debian / Ubuntu (APT)
Install via our official Debian package repository to get automatic updates:

```bash
# Add official signing repository
echo "deb [signed-by=/usr/share/keyrings/crush.gpg] https://apt.crushrun.dev/ stable main" | sudo tee /etc/apt/sources.list.d/crush.list

# Update package feeds and install
sudo apt update && sudo apt install crush
```

### Fedora / Red Hat / Rocky (RPM)
Configure the repository feed and install using `dnf` or `yum`:

```bash
# Configure system package feed
sudo dnf config-manager --add-repo https://rpm.crushrun.dev/crush.repo

# Install binary suite
sudo dnf install crush
```

### Arch Linux (AUR)
Install using your preferred AUR helper:

```bash
# Using yay
yay -S crush-bin

# Using paru
paru -S crush-bin
```

---

## Windows & macOS Package Managers

### Windows (Winget & Scoop)
You can install Crush on Windows natively using Winget or Scoop:

```powershell
# Using winget
winget install Chidi09.Crush

# Using scoop
scoop bucket add crush https://github.com/Chidi09/crush-bucket.git
scoop install crush
```

### macOS (Homebrew)
Install using Homebrew:

```bash
brew install Chidi09/crush/crush
```

---

## Rust Cargo (Build from Source)
If you have the Rust toolchain installed, you can build and install Crush from source:

```bash
cargo install --locked crush-cli
```

---

## Verification

To verify that your installation was successful and is correctly linked in your path, run:

```bash
crush --version
```
