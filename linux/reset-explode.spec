Name: reset-explode-gnome
Version: 0.32.0
Release: 1%{?dist}
Summary: Reset Explode para Linux
License: GPL-3.0-only
BuildArch: x86_64
AutoReqProv: no
Obsoletes: reset-explode-gnome-preview < 0.27.0
Provides: reset-explode-gnome-preview = %{version}-%{release}
Requires: gtk4, libadwaita, glib2, glibc >= 2.39, hwdata

%description
Aplicativo em Rust para consultar informacoes e sensores do sistema.

%install
mkdir -p %{buildroot}/usr/bin %{buildroot}/usr/share/applications %{buildroot}/usr/share/icons/hicolor/256x256/apps %{buildroot}/usr/share/doc/%{name}
install -m 755 %{_sourcedir}/reset-explode-linux %{buildroot}/usr/bin/reset-explode-linux
install -m 644 %{_sourcedir}/reset-explode-linux.desktop %{buildroot}/usr/share/applications/
install -m 644 %{_sourcedir}/reset-explode-linux.png %{buildroot}/usr/share/icons/hicolor/256x256/apps/
install -m 644 %{_sourcedir}/LICENSE %{_sourcedir}/README.md %{buildroot}/usr/share/doc/%{name}/

%files
/usr/bin/reset-explode-linux
/usr/share/applications/reset-explode-linux.desktop
/usr/share/icons/hicolor/256x256/apps/reset-explode-linux.png
/usr/share/doc/%{name}/LICENSE
/usr/share/doc/%{name}/README.md
