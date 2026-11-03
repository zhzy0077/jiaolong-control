#!/usr/bin/env python3
"""Build native x86_64 RPM/DEB packages with a DKMS module source payload."""
import argparse
from pathlib import Path
import platform
import re
import shutil
import subprocess
import tarfile
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parent.parent


def run(*args):
    subprocess.run(args, cwd=ROOT, check=True)


def copy(source, destination, mode=0o644):
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)
    destination.chmod(mode)


def template(source, destination, version, mode=0o644):
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(source.read_text().replace('@VERSION@', version))
    destination.chmod(mode)


def stage(payload, binary, version):
    copy(binary, payload / 'usr/bin/jiaolong-control', 0o755)
    copy(ROOT / 'packaging/kernel-module', payload / 'usr/lib/jiaolong-control/kernel-module', 0o755)
    copy(ROOT / 'linux/70-jiaolong-mifs.rules', payload / 'usr/lib/udev/rules.d/70-jiaolong-mifs.rules')
    modules_load = payload / 'usr/lib/modules-load.d/jiaolong-mifs.conf'
    modules_load.parent.mkdir(parents=True, exist_ok=True)
    modules_load.write_text('jiaolong_mifs\n')
    source = payload / f'usr/src/jiaolong-mifs-{version}'
    for name in ('Makefile', 'jiaolong_mifs.c', 'COPYING'):
        copy(ROOT / 'linux' / name, source / name)
    template(ROOT / 'packaging/dkms.conf.in', source / 'dkms.conf', version)
    copy(ROOT / 'packaging/jiaolong-control.desktop', payload / 'usr/share/applications/jiaolong-control.desktop')
    copy(ROOT / 'packaging/jiaolong-control.svg', payload / 'usr/share/icons/hicolor/scalable/apps/jiaolong-control.svg')
    copy(ROOT / 'README.md', payload / 'usr/share/doc/jiaolong-control/README.md')
    for source, name in ((ROOT / 'linux/COPYING', 'kernel-GPL-2.0'),
                         (ROOT / 'licenses/GPUI-APACHE-2.0.txt', 'GPUI-APACHE-2.0')):
        copy(source, payload / 'usr/share/licenses/jiaolong-control' / name)


def build_rpm(work, payload, output, version):
    top = work / 'rpm'
    for name in ('SOURCES', 'SPECS', 'BUILD', 'BUILDROOT', 'RPMS', 'SRPMS'):
        (top / name).mkdir(parents=True)
    with tarfile.open(top / f'SOURCES/jiaolong-control-{version}.tar.gz', 'w:gz') as archive:
        archive.add(payload, arcname=f'jiaolong-control-{version}')
    spec = top / 'SPECS/jiaolong-control.spec'
    copy(ROOT / 'packaging/jiaolong-control.spec', spec)
    run('rpmbuild', '-bb', '--define', f'_topdir {top}', '--define', f'_app_version {version}', str(spec))
    for package in (top / 'RPMS').rglob('*.rpm'):
        copy(package, output / package.name)


def build_deb(work, payload, output, binary, version):
    package = work / 'deb'
    shutil.copytree(payload, package)
    control = package / 'DEBIAN'
    control.mkdir()
    symbols = subprocess.check_output(['readelf', '--version-info', str(binary)], text=True)
    required = re.findall(r'Name: GLIBC_(\d+\.\d+(?:\.\d+)?)\s+Flags: none', symbols)
    glibc = max(required, key=lambda value: tuple(map(int, value.split('.'))))
    installed_size = sum(path.stat().st_size for path in payload.rglob('*') if path.is_file()) // 1024
    (control / 'control').write_text(f'''Package: jiaolong-control
Version: {version}-1
Section: utils
Priority: optional
Architecture: amd64
Maintainer: zhzy0077 <zhzy0077@users.noreply.github.com>
Installed-Size: {installed_size}
Depends: dkms, build-essential, linux-headers-amd64 | linux-headers-generic, kmod, udev, libc6 (>= {glibc}), libgcc-s1, libxcb1 (>= 1.12), libxkbcommon0 (>= 0.5.0), libxkbcommon-x11-0 (>= 0.5.0), libfontconfig1, libwayland-client0, libvulkan1, libegl1
Homepage: https://github.com/zhzy0077/jiaolong-control
Description: Fan and performance mode control for Jiaolong laptops
 Rust/GPUI desktop app using the MIFS WMI interface. Includes a DKMS
 kernel module built and installed by the package post-install script.
''')
    for name in ('postinst', 'prerm'):
        template(ROOT / f'packaging/debian/{name}.in', control / name, version, 0o755)
    copy(ROOT / 'packaging/debian/postrm', control / 'postrm', 0o755)
    run('dpkg-deb', '--root-owner-group', '--build', str(package), str(output / f'jiaolong-control_{version}-1_amd64.deb'))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('format', choices=('rpm', 'deb', 'all'), nargs='?', default='all')
    parser.add_argument('--binary', type=Path, help='Package an existing release executable')
    args = parser.parse_args()
    if platform.machine() != 'x86_64':
        parser.error('This package builder currently targets x86_64/amd64.')
    version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']['version']
    if not re.fullmatch(r'\d+\.\d+\.\d+', version):
        parser.error('Package version must use MAJOR.MINOR.PATCH.')
    if args.binary:
        binary = args.binary.resolve(strict=True)
    else:
        run('cargo', 'build', '--release', '--locked')
        binary = ROOT / 'target/release/jiaolong-control'
    output = ROOT / 'dist/packages'
    output.mkdir(parents=True, exist_ok=True)
    (ROOT / 'target/packages').mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=ROOT / 'target/packages') as temporary:
        work = Path(temporary)
        payload = work / 'payload'
        stage(payload, binary, version)
        if args.format in ('rpm', 'all'):
            build_rpm(work, payload, output, version)
        if args.format in ('deb', 'all'):
            build_deb(work, payload, output, binary, version)
    packages = sorted(output.glob('*.rpm')) + sorted(output.glob('*.deb'))
    import hashlib
    checksums = []
    for path in packages:
        with path.open('rb') as file:
            checksums.append(f'{hashlib.file_digest(file, "sha256").hexdigest()}  {path.name}\n')
    (output / 'SHA256SUMS').write_text(''.join(checksums))
    for path in packages:
        print(path)


if __name__ == '__main__':
    main()
