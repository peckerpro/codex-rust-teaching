# Agent Operations: SSH, Linux Testing, and Git Push

This project is Windows-hosted in the current Codex desktop thread, but the
primary build and test environment is the local Linux VM.

Do not commit secrets. All passwords, tokens, local SSH keys, and helper scripts
must stay under `.local-env/`, which is ignored by Git.

## 1. Local Secret Files

Expected ignored files:

```text
.local-env/rust-teaching.ps1
.local-env/rust-teaching.env
.local-env/codex_vm_ed25519_nopass
.local-env/codex_vm_ed25519_nopass.pub
.local-env/git-askpass.sh
```

PowerShell variables used on Windows:

```powershell
. .\.local-env\rust-teaching.ps1
```

Expected variables:

```text
RTC_LINUX_HOST
RTC_LINUX_USER
RTC_LINUX_SSH_KEY
RTC_LINUX_SUDO_PASSWORD
RTC_GITHUB_REPO
RTC_GITHUB_TOKEN
```

Linux shell variables used on the VM:

```bash
source .local-env/rust-teaching.env
```

## 2. SSH Into the VM

From the Windows project root:

```powershell
. .\.local-env\rust-teaching.ps1
ssh -i $env:RTC_LINUX_SSH_KEY `
  -o IdentitiesOnly=yes `
  -o BatchMode=yes `
  -o StrictHostKeyChecking=accept-new `
  "$env:RTC_LINUX_USER@$env:RTC_LINUX_HOST"
```

Run a one-off command:

```powershell
. .\.local-env\rust-teaching.ps1
ssh -i $env:RTC_LINUX_SSH_KEY `
  -o IdentitiesOnly=yes `
  -o BatchMode=yes `
  -o StrictHostKeyChecking=accept-new `
  "$env:RTC_LINUX_USER@$env:RTC_LINUX_HOST" `
  'source /home/pecker/.cargo/env; cd /home/pecker/codex-project/rust-teaching; cargo test'
```

If key auth fails, force password auth once and repair `authorized_keys`:

```powershell
ssh -o PubkeyAuthentication=no `
  -o PreferredAuthentications=password `
  -o StrictHostKeyChecking=accept-new `
  pecker@192.168.226.130
```

## 3. Primary Linux Validation

Use the VM as the source of truth for build and test validation:

```bash
source /home/pecker/.cargo/env
cd /home/pecker/codex-project/rust-teaching
cargo fmt --all --check
cargo test
cargo run -p rt-cli -- --emit tokens --format json examples/basic.rs
cargo run -p rt-cli -- --emit ast --format json examples/basic.rs
cargo run -p rt-cli -- --emit semantic --format json examples/basic.rs
cargo run -p rt-cli -- -S examples/basic.rs -o examples/basic.ll
opt-18 -passes=verify examples/basic.ll -disable-output
lli-18 examples/basic.ll
```

LLVM 18 uses the new pass manager syntax. Prefer:

```bash
opt-18 -passes=verify examples/basic.ll -disable-output
```

instead of the older:

```bash
opt-18 -verify examples/basic.ll -disable-output
```

## 4. Sync Windows Work to the VM

If edits are made in the Windows workspace, transfer commits to the VM with a
bundle:

```powershell
git bundle create .local-env\rust-teaching-main.bundle main

. .\.local-env\rust-teaching.ps1
scp -i $env:RTC_LINUX_SSH_KEY `
  -o IdentitiesOnly=yes `
  -o BatchMode=yes `
  -o StrictHostKeyChecking=accept-new `
  .local-env\rust-teaching-main.bundle `
  "$env:RTC_LINUX_USER@$env:RTC_LINUX_HOST:/home/pecker/codex-project/rust-teaching/.local-env/rust-teaching-main.bundle"
```

On the VM:

```bash
cd /home/pecker/codex-project/rust-teaching
git fetch .local-env/rust-teaching-main.bundle main
git merge --ff-only FETCH_HEAD
```

If the VM worktree already has the same file content but the branch pointer needs
to be advanced to the bundled commit:

```bash
git fetch .local-env/rust-teaching-main.bundle main
git diff --quiet FETCH_HEAD --
git update-ref refs/heads/main FETCH_HEAD
git reset --mixed HEAD
```

Only use the `update-ref` path after confirming `git diff --quiet FETCH_HEAD --`
succeeds.

## 5. Git Push From the VM

The VM usually has better GitHub connectivity than the Windows sandbox. Push from
the VM using the ignored askpass helper:

```bash
cd /home/pecker/codex-project/rust-teaching
source .local-env/rust-teaching.env
GIT_ASKPASS="$PWD/.local-env/git-askpass.sh" \
GIT_TERMINAL_PROMPT=0 \
git push origin main
```

The askpass helper should read `RTC_GITHUB_TOKEN` from the environment and should
not contain the token itself:

```sh
#!/bin/sh
case "$1" in
  *Username*) printf '%s\n' 'x-access-token' ;;
  *) printf '%s\n' "$RTC_GITHUB_TOKEN" ;;
esac
```

## 6. Codex Remote Connections Note

Codex Settings -> Connections -> Add SSH connection is useful when a new Codex
thread or project is opened directly against that remote environment. It does
not automatically migrate an already-running Windows-local thread whose `cwd` is
`D:\codex-project\rust-teaching`.

For the current thread shape, keep using explicit SSH commands and treat the VM
as the main validation host.

