# Linux packages

Build on the target distribution with Python 3.11+, Rust, its graphics development libraries, and `rpm-build` (RPM) or `dpkg` (DEB):

```bash
python3 packaging/build.py rpm
python3 packaging/build.py deb
```

Output is in `dist/packages/`, with `SHA256SUMS`. `--binary PATH` packages an already-built executable. Build the DEB executable on Debian 12 or another intended Debian/Ubuntu target so its glibc symbols match that distribution. RPM dependencies are extracted from the executable; DEB records the required glibc version and graphics libraries.

Install with dependency resolution:

```bash
sudo dnf install ./dist/packages/jiaolong-control-*.rpm
sudo apt install ./dist/packages/jiaolong-control_*_amd64.deb
```

Both packages install a desktop launcher and icon, the application, udev access rules, an auto-load configuration, and module source under `/usr/src/jiaolong-mifs-VERSION/`. Their post-install scripts register the source with DKMS, build/install it for kernels with development headers, and load it on supported hardware when the running kernel has headers. DKMS rebuilds the module after kernel updates.

RPM targets Fedora and depends on `kernel-devel-matched`. DEB depends on `linux-headers-amd64` or `linux-headers-generic`; custom kernels need their matching headers. If only a newer installed kernel has headers, the script builds for it and reports that a reboot is required. Close the app before upgrading its loaded module. Secure Boot module loading requires a trusted DKMS signing key.

Removal unregisters the package's DKMS version and removes package-owned access rules and auto-load configuration. User preferences and fan targets remain under the user's `jiaolong-control` configuration directory.

Package install/remove commands disable DKMS's global `post_transaction` hook for those invocations. This module loads after boot, so these operations require `depmod` and `modprobe` rather than rebuilding every initramfs. System-wide DKMS configuration is preserved.

Hook tests use fake kernels and command stubs:

```bash
python3 -m unittest discover -s tests -p test_packaging.py
```
