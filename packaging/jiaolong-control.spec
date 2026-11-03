%global debug_package %{nil}
%global _build_id_links none
Name:           jiaolong-control
Version:        %{_app_version}
Release:        1%{?dist}
Summary:        Fan and performance mode control for Jiaolong laptops
License:        LicenseRef-NotSpecified AND GPL-2.0-only AND Apache-2.0
URL:            https://github.com/zhzy0077/jiaolong-control
Source0:        %{name}-%{version}.tar.gz
Requires:       dkms, gcc, make, kernel-devel-matched, kmod, systemd-udev
Requires:       fontconfig, libwayland-client, vulkan-loader, mesa-libEGL

%description
Rust/GPUI desktop fan control for Jiaolong laptops with the MIFS WMI interface.
Includes a DKMS kernel module, built and installed by the package post-script.

%prep
%setup -q

%build
# The release executable is built by packaging/build.py.

%install
mkdir -p %{buildroot}
cp -a ./* %{buildroot}/

%post
/usr/lib/jiaolong-control/kernel-module install %{version}

%preun
if [ "$1" -eq 0 ]; then
    /usr/lib/jiaolong-control/kernel-module remove %{version}
fi

%postun
if [ "$1" -eq 0 ] && [ -S /run/udev/control ]; then
    udevadm control --reload-rules
fi

%files
/usr/bin/jiaolong-control
/usr/lib/jiaolong-control/
/usr/lib/udev/rules.d/70-jiaolong-mifs.rules
/usr/lib/modules-load.d/jiaolong-mifs.conf
/usr/src/jiaolong-mifs-%{version}/
/usr/share/applications/jiaolong-control.desktop
/usr/share/icons/hicolor/scalable/apps/jiaolong-control.svg
%doc /usr/share/doc/jiaolong-control/README.md
%license /usr/share/licenses/jiaolong-control/*
