Name: reset-explode-gnome-preview
Version: 0.13.0
Release: 1%{?dist}
Summary: Previa Linux do Reset Explode
License: MIT
BuildArch: x86_64
AutoReqProv: no
Requires: gtk4, libadwaita, glib2, glibc >= 2.39, hwdata

%description
Previa em Rust para consultar informacoes e sensores do sistema.

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
